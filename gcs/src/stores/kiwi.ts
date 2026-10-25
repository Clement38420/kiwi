import { defineStore } from 'pinia'
import type { Message } from '@/bindings/Message.ts'
import { decode } from '@msgpack/msgpack'
import { shallowRef } from 'vue'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'

export const useKiwiStore = defineStore('kiwi', () => {
  const websocket = new WebSocket(`/ws`)
  websocket.binaryType = 'arraybuffer'

  const lidarDataPoints = shallowRef<LidarPoint[]>([])

  function dispatchMessage(message: Message) {
    if ('LidarScan' in message) {
      console.log('LidarDataPoints')
      lidarDataPointsHandler(message.LidarScan)
    } else if ('WheelSpeedSetpoints' in message) {
      // nothing to do for now
    } else {
      const _exhaustiveCheck: never = message
      return _exhaustiveCheck
    }
  }

  function lidarDataPointsHandler(dataPoints: LidarPoint[]) {
    lidarDataPoints.value = dataPoints
  }

  websocket.onmessage = (event) => {
    const msg = decode(event.data) as Message
    dispatchMessage(msg)
  }

  return {
    lidarDataPoints,
  }
})
