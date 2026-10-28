import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode } from '@msgpack/msgpack'
import { ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'

export const useRobotStore = defineStore('robot', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const connectionStatus = ref(websocket.readyState)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const batteryVoltage = ref<number>(0)

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
      lidarScanHandler(message.LidarScan)
    } else if ('BatteryVoltage' in message) {
      batteryVoltageHandler(message.BatteryVoltage)
    } else {
      const _exhaustiveCheck: never = message
      return _exhaustiveCheck
    }
  }

  function lidarScanHandler(dataPoints: LidarPoint[]) {
    lidarScan.value = dataPoints
  }

  function batteryVoltageHandler(voltage: number) {
    batteryVoltage.value = voltage
  }

  return {
    lidarScan,
    batteryVoltage,
    connectionStatus,
  }
})
