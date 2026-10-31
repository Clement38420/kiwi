import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode, encode } from '@msgpack/msgpack'
import { computed, ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { GcsToRpi } from '@/bindings/GcsToRpi.ts'
import { createRpcClient } from '@/utils/rpc.ts'
import type { RobotState } from '@/bindings/RobotState.ts'

export const useRobotStore = defineStore('robot', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const connectionState = ref(websocket.readyState)
  const isConnected = computed(() => connectionState.value === WebSocket.OPEN)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const robotState = ref<RobotState>({
    battery_voltage: 0,
    battery_level: 'Critical',
    arm: 'CannotArm',
  })

  const rpc = createRpcClient((m) => {
    websocket.send(encode(m))
  })

  const sendRequest = rpc.request

  websocket.onopen = async () => {
    connectionState.value = websocket.readyState
    try {
      robotState.value = await rpc.request('GetState')
    } catch (e) {
      console.warn(e)
    }
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
    if (rpc.handleMessage(message)) {
      return
    } else if ('LidarScan' in message) {
      lidarScan.value = message.LidarScan
    } else if ('BatteryVoltage' in message) {
      robotState.value.battery_voltage = message.BatteryVoltage
    } else if ('BatteryLevel' in message) {
      robotState.value.battery_level = message.BatteryLevel
    } else if ('ArmState' in message) {
      robotState.value.arm = message.ArmState
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
    connectionState,
    robotState,
    sendMessage,
    isConnected,
    sendRequest,
  }
})
