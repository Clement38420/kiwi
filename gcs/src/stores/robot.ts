import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode } from '@msgpack/msgpack'
import { ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { BatteryState } from '@/bindings/BatteryState.ts'

export const useRobotStore = defineStore('robot', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const connectionState = ref(websocket.readyState)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const batteryVoltage = ref<number>(0)
  const batteryState = ref<BatteryState>('High')

  websocket.onopen = () => {
    connectionState.value = websocket.readyState
  }

  websocket.onclose = () => {
    connectionState.value = websocket.readyState
  }

  websocket.onerror = () => {
    connectionState.value = websocket.readyState
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
    } else if ('BatteryState' in message) {
      batteryState.value = message.BatteryState
    } else {
      const _exhaustiveCheck: never = message
      return _exhaustiveCheck
    }
  }

  return {
    lidarScan,
    batteryVoltage,
    connectionState,
    batteryState,
  }
})
