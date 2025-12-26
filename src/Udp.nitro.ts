import { type HybridObject } from 'react-native-nitro-modules'

/**
 * @specs
 */
export interface UdpSocketDriver extends HybridObject<{ ios: 'swift', android: 'kotlin' }> {
    readonly id: number
    bind(port: number, address: string, ipv6Only: boolean): number
    connect(port: number, address: string): number
    disconnect(): number
    send(data: ArrayBuffer, port: number, address: string): number
    sendMultiple(data: ArrayBuffer[], port: number, address: string): number
    close(): void

    setBroadcast(flag: boolean): number
    setTTL(ttl: number): number
    setMulticastTTL(ttl: number): number
    setMulticastLoopback(flag: boolean): number
    setMulticastInterface(interfaceAddress: string): number
    addMembership(multicastAddress: string, interfaceAddress?: string): number
    dropMembership(multicastAddress: string, interfaceAddress?: string): number
    addSourceSpecificMembership(sourceAddress: string, groupAddress: string, interfaceAddress?: string): number
    dropSourceSpecificMembership(sourceAddress: string, groupAddress: string, interfaceAddress?: string): number

    getLocalAddress(): string
    getLocalPort(): number
    getRemoteAddress(): string
    getRemotePort(): number

    getRecvBufferSize(): number
    setRecvBufferSize(size: number): number
    getSendBufferSize(): number
    setSendBufferSize(size: number): number
    getSendQueueCount(): number
    getSendQueueSize(): number

    onMessage: (data: ArrayBuffer, address: string, port: number) => void
    onConnect: () => void
    onError: (error: string) => void
    onClose: () => void
}

export interface RemoteInfo {
    address: string
    family: 'IPv4' | 'IPv6'
    port: number
    size: number
}

/**
 * @specs
 */
export interface UdpDriver extends HybridObject<{ ios: 'swift', android: 'kotlin' }> {
    createSocket(id?: string): UdpSocketDriver
}
