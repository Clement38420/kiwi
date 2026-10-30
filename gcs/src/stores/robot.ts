import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode, encode } from '@msgpack/msgpack'
import { computed, ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { BatteryState } from '@/bindings/BatteryState.ts'
import type { GcsToRpi } from '@/bindings/GcsToRpi.ts'
import type { ArmState } from '@/bindings/ArmState.ts'

export const useRobotStore = defineStore('robot', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const connectionState = ref(websocket.readyState)
  const isConnected = computed(() => connectionState.value === WebSocket.OPEN)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const batteryVoltage = ref<number>(0)
  const batteryState = ref<BatteryState>('High')
  const armState = ref<ArmState>('Disarmed')

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
    } else if ('ArmState' in message) {
      armState.value = message.ArmState
    } else {
      const _exhaustiveCheck: never = message
      return _exhaustiveCheck
    }
  }

  function sendMessage(message: GcsToRpi) {
    websocket.send(encode(message))
  }

  return {
    lidarScan,
    batteryVoltage,
    connectionState,
    batteryState,
    sendMessage,
    armState,
    isConnected,
  }
})
