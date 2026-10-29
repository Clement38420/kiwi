import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode } from '@msgpack/msgpack'
import { ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { BatteryStatus } from '@/bindings/BatteryStatus.ts'

export const useRobotStore = defineStore('robot', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const connectionStatus = ref(websocket.readyState)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const batteryVoltage = ref<number>(0)
  const batteryStatus = ref<BatteryStatus>('High')

  websocket.onopen = () => {
    connectionStatus.value = websocket.readyState
  }

  websocket.onclose = () => {
    connectionStatus.value = websocket.readyState
  }

  websocket.onerror = () => {
    connectionStatus.value = websocket.readyState
  }

  websocket.onmessage = (event) => {
    const msg = decode(event.data) as RpiToGcs
    dispatchMessage(msg)
  }

  function dispatchMessage(message: RpiToGcs) {
    if ('LidarScan' in message) {
      lidarScan.value = message.LidarScan
    } else if ('BatteryVoltage' in message) {
      batteryVoltage.value = message.BatteryVoltage
    } else if ('BatteryStatus' in message) {
      batteryStatus.value = message.BatteryStatus
    } else {
      const _exhaustiveCheck: never = message
      return _exhaustiveCheck
    }
  }

  return {
    lidarScan,
    batteryVoltage,
    connectionStatus,
    batteryStatus,
  }
})
