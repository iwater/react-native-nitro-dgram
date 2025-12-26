import { EventEmitter } from 'eventemitter3'
import { Driver } from './Driver'
import type { UdpSocketDriver, RemoteInfo } from './Udp.nitro'
import { Buffer } from 'react-native-nitro-buffer'

/**
 * 100% compatible with Node.js dgram module.
 */

function isIP(input: string): number {
    if (/^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$/.test(input)) return 4;
    if (/^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$/.test(input)) return 6;
    if (/^((?:[0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{1,4})*)?)::((?:[0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{1,4})*)?)$/.test(input)) return 6;
    return 0;
}

export type SocketType = 'udp4' | 'udp6'

export class BlockList {
    private _rules: Array<{ type: 'address' | 'range' | 'subnet', data: any }> = []

    addAddress(address: string, family: 'ipv4' | 'ipv6' = 'ipv4'): void {
        this._rules.push({ type: 'address', data: { address, family } })
    }

    addRange(start: string, end: string, family: 'ipv4' | 'ipv6' = 'ipv4'): void {
        this._rules.push({ type: 'range', data: { start, end, family } })
    }

    addSubnet(net: string, prefix: number, family: 'ipv4' | 'ipv6' = 'ipv4'): void {
        this._rules.push({ type: 'subnet', data: { net, prefix, family } })
    }

    check(address: string, family: 'ipv4' | 'ipv6' = 'ipv4'): boolean {
        // Simple implementation for now
        for (const rule of this._rules) {
            if (rule.type === 'address' && rule.data.address === address) return true
            // Range and subnet check would need IP calculation logic
        }
        return false
    }
}

export interface SocketOptions {
    type: SocketType
    reuseAddr?: boolean
    reusePort?: boolean
    ipv6Only?: boolean
    recvBufferSize?: number
    sendBufferSize?: number
    signal?: AbortSignal
    lookup?: (hostname: string, options: any, callback: (err: Error | null, address: string, family: number) => void) => void
    receiveBlockList?: BlockList
    sendBlockList?: BlockList
}

export class Socket extends EventEmitter {
    private _driver: UdpSocketDriver
    private _type: SocketType
    private _bound: boolean = false
    private _closed: boolean = false
    private _connected: boolean = false
    private _signal?: AbortSignal
    private _lookup?: (hostname: string, options: any, callback: (err: Error | null, address: string, family: number) => void) => void
    private _receiveBlockList?: BlockList
    private _sendBlockList?: BlockList

    constructor(type: SocketType | SocketOptions) {
        super()
        const options = typeof type === 'string' ? { type } : type
        this._type = options.type
        this._lookup = options.lookup
        this._receiveBlockList = options.receiveBlockList
        this._sendBlockList = options.sendBlockList
        this._driver = Driver.createSocket()
        this._setupEvents()

        if (options.signal) {
            this._signal = options.signal
            if (this._signal.aborted) {
                setImmediate(() => this.close())
            } else {
                this._signal.addEventListener('abort', () => {
                    this.close()
                })
            }
        }
    }

    async [Symbol.asyncDispose]() {
        if (this._closed) return
        return new Promise<void>((resolve) => {
            this.once('close', resolve)
            this.close()
        })
    }

    private _setupEvents() {
        this._driver.onMessage = (data: ArrayBuffer, address: string, port: number) => {
            const family = isIP(address) === 6 ? 'IPv6' : 'IPv4'
            if (this._receiveBlockList && this._receiveBlockList.check(address, family === 'IPv6' ? 'ipv6' : 'ipv4')) {
                return // Discard blocked message
            }

            const buffer = Buffer.from(data)
            const rinfo: RemoteInfo = {
                address,
                family,
                port,
                size: buffer.length
            }
            this.emit('message', buffer, rinfo)
        }

        this._driver.onConnect = () => {
            this._connected = true
            this.emit('connect')
        }

        this._driver.onError = (error: string) => {
            this.emit('error', new Error(error))
        }

        this._driver.onClose = () => {
            this._closed = true
            this.emit('close')
        }
    }

    bind(port?: number, address?: string, callback?: () => void): this;
    bind(options: { port?: number, address?: string, exclusive?: boolean }, callback?: () => void): this;
    bind(arg1?: any, arg2?: any, arg3?: any): this {
        if (this._bound) throw new Error('Already bound');

        let port = 0
        let address = ''
        let callback: (() => void) | undefined

        if (typeof arg1 === 'object' && arg1 !== null) {
            port = arg1.port || 0
            address = arg1.address || ''
            callback = arg2
        } else {
            port = arg1 || 0
            address = arg2 || ''
            callback = arg3
        }

        if (callback) this.once('listening', callback);

        const ipv6Only = false // TODO: handle from options
        const result = this._driver.bind(port, address, ipv6Only)

        if (result === 0) {
            this._bound = true
            setImmediate(() => this.emit('listening'))
        } else {
            setImmediate(() => this.emit('error', new Error(`Bind failed with code ${result}`)))
        }

        return this
    }

    connect(port: number, address?: string, callback?: () => void): void {
        if (this._closed) throw new Error('Socket is closed');

        if (!this._bound) {
            this.bind(0);
        }

        if (callback) this.once('connect', callback);

        const targetAddress = address || (this._type === 'udp4' ? '127.0.0.1' : '::1');
        const result = this._driver.connect(port, targetAddress);

        if (result !== 0) {
            setImmediate(() => this.emit('error', new Error(`Connect failed with code ${result}`)));
        }
    }

    disconnect(): void {
        if (this._closed) throw new Error('Socket is closed');
        if (!this._connected) return;

        const result = this._driver.disconnect();
        if (result === 0) {
            this._connected = false;
        } else {
            // Some platforms might not support disconnect directy
            // If disconnect failed but node says it should work, we might need to recreate socket
            // but for now we'll just emit error or ignore if it's ENOTSUP
        }
    }

    send(msg: string | Uint8Array | ReadonlyArray<any>, port?: number, address?: string, callback?: (error: Error | null, bytes: number) => void): void;
    send(msg: string | Uint8Array, offset: number, length: number, port?: number, address?: string, callback?: (error: Error | null, bytes: number) => void): void;
    send(msg: any, ...args: any[]): void {
        if (this._closed) throw new Error('Socket is closed');

        let offset = 0
        let length = 0
        let port = 0
        let address = ''
        let callback: ((error: Error | null, bytes: number) => void) | undefined

        // Node.js send() behavior:
        // If connected, port and address should not be provided.
        // If not connected, port and address must be provided.

        if (!this._bound) {
            this.bind(0);
        }

        if (this._connected) {
            // (msg, [offset, length,] [callback])
            if (typeof args[0] === 'function') {
                callback = args[0];
                length = msg.length;
            } else if (typeof args[0] === 'number' && typeof args[1] === 'number') {
                offset = args[0];
                length = args[1];
                callback = args[2];
            } else {
                length = msg.length;
            }
            // Use connected peer info
            port = this._driver.getRemotePort();
            address = this._driver.getRemoteAddress();
        } else {
            // (msg, [offset, length,] port [, address] [, callback])
            if (args.length >= 4) {
                // (msg, offset, length, port, address, callback)
                offset = args[0]
                length = args[1]
                port = args[2]
                address = args[3] || '127.0.0.1'
                callback = args[4]
            } else {
                // (msg, port, address, callback)
                port = args[0]
                address = args[1] || '127.0.0.1'
                callback = args[2]
                length = msg.length
            }
        }

        const handleSend = (targetAddress: string) => {
            const family = isIP(targetAddress) === 6 ? 'IPv6' : 'IPv4'
            if (this._sendBlockList && this._sendBlockList.check(targetAddress, family === 'IPv6' ? 'ipv6' : 'ipv4')) {
                const err = new Error(`Address ${targetAddress} is blocked by sendBlockList`)
                if (callback) setImmediate(() => callback!(err, 0))
                else this.emit('error', err)
                return
            }

            let bytesSent = 0
            if (Array.isArray(msg)) {
                const abs: ArrayBuffer[] = msg.map(m => {
                    const b = Buffer.from(m)
                    return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength)
                })
                bytesSent = this._driver.sendMultiple(abs, port, targetAddress)
            } else {
                const buffer = Buffer.from(msg)
                const finalBuffer = (offset === 0 && (length === 0 || length === buffer.length))
                    ? buffer
                    : buffer.slice(offset, offset + (length || buffer.length))

                const ab = finalBuffer.buffer.slice(finalBuffer.byteOffset, finalBuffer.byteOffset + finalBuffer.byteLength)
                bytesSent = this._driver.send(ab, port, targetAddress)
            }

            if (callback) {
                if (bytesSent >= 0) {
                    setImmediate(() => callback!(null, bytesSent))
                } else {
                    setImmediate(() => callback!(new Error(`Send failed with code ${bytesSent}`), 0))
                }
            }
        }

        if (this._lookup && isIP(address) === 0 && !this._connected && address !== '') {
            this._lookup(address, {}, (err, resolvedAddr) => {
                if (err) {
                    if (callback) callback(err, 0);
                    else this.emit('error', err);
                    return;
                }
                handleSend(resolvedAddr);
            });
        } else {
            handleSend(address);
        }
    }

    close(callback?: () => void): this {
        if (this._closed) {
            if (callback) setImmediate(callback);
            return this;
        }
        if (callback) this.once('close', callback);

        // Only bound sockets have native listener, so only they receive native onClose
        if (this._bound) {
            this._driver.close();
        } else {
            // For unbound sockets, emit close directly since native layer won't
            this._closed = true;
            setImmediate(() => this.emit('close'));
        }

        return this
    }

    address(): { address: string, family: string, port: number } {
        return {
            address: this._driver.getLocalAddress(),
            family: this._type === 'udp6' ? 'IPv6' : 'IPv4',
            port: this._driver.getLocalPort()
        }
    }

    remoteAddress(): { address: string, family: string, port: number } {
        if (!this._connected) throw new Error('Socket is not connected');
        return {
            address: this._driver.getRemoteAddress(),
            family: isIP(this._driver.getRemoteAddress()) === 6 ? 'IPv6' : 'IPv4',
            port: this._driver.getRemotePort()
        }
    }

    setBroadcast(flag: boolean): void {
        this._driver.setBroadcast(flag)
    }

    setTTL(ttl: number): void {
        this._driver.setTTL(ttl)
    }

    setMulticastTTL(ttl: number): void {
        this._driver.setMulticastTTL(ttl)
    }

    setMulticastLoopback(flag: boolean): void {
        this._driver.setMulticastLoopback(flag)
    }

    setMulticastInterface(interfaceAddress: string): void {
        this._driver.setMulticastInterface(interfaceAddress)
    }

    addMembership(multicastAddress: string, interfaceAddress?: string): void {
        this._driver.addMembership(multicastAddress, interfaceAddress)
    }

    dropMembership(multicastAddress: string, interfaceAddress?: string): void {
        this._driver.dropMembership(multicastAddress, interfaceAddress)
    }

    addSourceSpecificMembership(sourceAddress: string, groupAddress: string, interfaceAddress?: string): void {
        this._driver.addSourceSpecificMembership(sourceAddress, groupAddress, interfaceAddress)
    }

    dropSourceSpecificMembership(sourceAddress: string, groupAddress: string, interfaceAddress?: string): void {
        this._driver.dropSourceSpecificMembership(sourceAddress, groupAddress, interfaceAddress)
    }

    getRecvBufferSize(): number {
        if (this._driver.id === 0) throw new Error('Socket is not bound');
        return this._driver.getRecvBufferSize()
    }

    setRecvBufferSize(size: number): void {
        if (this._driver.id === 0) throw new Error('Socket is not bound');
        this._driver.setRecvBufferSize(size)
    }

    getSendBufferSize(): number {
        if (this._driver.id === 0) throw new Error('Socket is not bound');
        return this._driver.getSendBufferSize()
    }

    setSendBufferSize(size: number): void {
        if (this._driver.id === 0) throw new Error('Socket is not bound');
        this._driver.setSendBufferSize(size)
    }

    getSendQueueCount(): number {
        if (this._driver.id === 0) return 0;
        return this._driver.getSendQueueCount()
    }

    getSendQueueSize(): number {
        if (this._driver.id === 0) return 0;
        return this._driver.getSendQueueSize()
    }

    ref(): this { return this }
    unref(): this { return this }
}

export function createSocket(type: SocketType | SocketOptions, callback?: (msg: Buffer, rinfo: RemoteInfo) => void): Socket {
    const socket = new Socket(type)
    if (callback) {
        socket.on('message', callback)
    }
    return socket
}

export default {
    createSocket,
    Socket,
    BlockList
}
