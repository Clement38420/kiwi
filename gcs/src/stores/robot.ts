import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode, encode } from '@msgpack/msgpack'
import { computed, ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { GcsToRpi } from '@/bindings/GcsToRpi.ts'
import { createRpcClient } from '@/utils/rpc.ts'
import type { RobotState } from '@/bindings/RobotState.ts'

const RECONNECT_DELAY_MS = 2000

export const useRobotStore = defineStore('robot', () => {
  const wsUrl = import.meta.env.DEV ? 'ws://kiwi.local:8080/ws' : '/ws'
  let websocket: WebSocket | null = null

  const connectionState = ref<number>(WebSocket.CONNECTING)
  const isConnected = computed(() => connectionState.value === WebSocket.OPEN)
  const lidarScan = shallowRef<LidarPoint[]>([])
  const robotState = ref<RobotState>({
    battery_voltage: 0,
    battery_level: 'Critical',
    arm: 'CannotArm',
  })

  const rpc = createRpcClient(sendMessage)

  const sendRequest = rpc.request

  function connect() {
    const ws = new WebSocket(wsUrl)
    ws.binaryType = 'arraybuffer'
    websocket = ws
    connectionState.value = ws.readyState

    ws.onopen = async () => {
      connectionState.value = ws.readyState
      try {
        robotState.value = await rpc.request('GetState')
      } catch (e) {
        console.warn(e)
      }
    }

    ws.onclose = () => {
      connectionState.value = ws.readyState
      rpc.rejectAll('ConnectionClosed')
      setTimeout(connect, RECONNECT_DELAY_MS)
    }

    ws.onerror = () => {
      connectionState.value = ws.readyState
    }

    ws.onmessage = (event) => {
      const msg = decode(event.data) as RpiToGcs
      dispatchMessage(msg)
    }
  }

  connect()

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
    if (websocket?.readyState === WebSocket.OPEN) {
      websocket.send(encode(message))
    }
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
