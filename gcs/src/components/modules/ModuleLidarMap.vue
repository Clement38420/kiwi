<script setup lang="ts">
import { House, ZoomIn, ZoomOut } from '@lucide/vue'
import { useRobotStore } from '@/stores/robot.ts'
import { useTemplateRef } from 'vue'
import { useViewport } from '@/composables/useViewport.ts'

const robotStore = useRobotStore()
const canvas = useTemplateRef<HTMLCanvasElement>('scan-canvas')

const { zoom, ppu, upd, resetViewport } = useViewport(canvas, (ctx) => {
  ctx.fillStyle = getComputedStyle(ctx.canvas).color
  robotStore.lidarScan.forEach((point) => {
    if (point.distance < 0.001) return

    const distance_px = point.distance * ppu.value
    const x = Math.sin(point.angle) * distance_px
    const y = -Math.cos(point.angle) * distance_px
    ctx.fillRect(x - 1, y - 1, 2, 2)
  })

  //const { x: poseX, y: poseY, theta } = robotStore.robotPose
  ctx.save()
  //ctx.translate(-poseY * ppu.value, -poseX * ppu.value)
  //ctx.rotate(-theta)
  ctx.fillStyle = '#84cc16'
  ctx.beginPath()
  ctx.moveTo(0, -8)
  ctx.lineTo(-5, 6)
  ctx.lineTo(5, 6)
  ctx.closePath()
  ctx.fill()
  ctx.restore()
})
</script>

<template>
  <div class="group relative w-auto h-auto p-4 flex flex-col">
    <div class="group-title">Lidar map</div>
    <div class="group absolute bg-background flex right-4 top-4 py-2 px-2 text-ink-muted">
      <button class="btn btn-icon size-9" @click="zoom *= 2">
        <ZoomIn />
      </button>
      <button class="btn btn-icon size-9" @click="zoom *= 0.5">
        <ZoomOut />
      </button>
      <button class="btn btn-icon size-9" @click="resetViewport">
        <House />
      </button>
    </div>
    <canvas ref="scan-canvas" class="bg-grid w-full flex-1 min-h-0 rounded-panel text-ink"></canvas>
    <div
      class="group absolute bottom-4 right-4 bg-background font-mono py-2 px-4 text-ink-muted text-sm"
    >
      <p>{{ upd }} m/div</p>
    </div>
  </div>
</template>

<style scoped></style>
