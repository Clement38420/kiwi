import { defineStore } from 'pinia'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import { decode, encode } from '@msgpack/msgpack'
import { computed, ref, shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'
import type { GcsToRpi } from '@/bindings/GcsToRpi.ts'
import { createRpcClient } from '@/utils/rpc.ts'
import type { RobotState } from '@/bindings/RobotState.ts'
import type { Pose2D } from '@/bindings/Pose2D.ts'

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
    mode: 'Manual',
  })
  const robotPose = ref<Pose2D>({ x: 0, y: 0, theta: 0 })

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
    } else if ('RobotState' in message) {
      robotState.value = message.RobotState
    } else if ('RobotPose' in message) {
      robotPose.value = message.RobotPose
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
    robotPose,
  }
})
