import { NitroModules } from 'react-native-nitro-modules'
import type { UdpDriver } from './Udp.nitro'

export const Driver = NitroModules.createHybridObject<UdpDriver>('UdpDriver')
