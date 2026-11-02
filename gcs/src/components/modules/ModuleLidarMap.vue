<script setup lang="ts">
import { House } from '@lucide/vue'
import { useRobotStore } from '@/stores/robot.ts'
import { useTemplateRef } from 'vue'
import { useViewport } from '@/composables/useViewport.ts'

const robotStore = useRobotStore()
const canvas = useTemplateRef<HTMLCanvasElement>('scan-canvas')

const { ppu, upd, resetViewport } = useViewport(canvas, (ctx) => {
  ctx.fillStyle = getComputedStyle(ctx.canvas).color
  robotStore.lidarScan.forEach((point) => {
    const angle_rad = (point.angle * Math.PI) / 180
    const distance_px = point.distance * 0.001 * ppu.value
    const x = Math.sin(angle_rad) * distance_px
    const y = -Math.cos(angle_rad) * distance_px
    ctx.fillRect(x - 1, y - 1, 2, 2)
  })

  ctx.fillStyle = '#84cc16'
  ctx.fillRect(-2, -2, 4, 4)
})
</script>

<template>
  <div class="group relative w-full h-full p-4 flex flex-col">
    <div class="group-title">Lidar map</div>
    <div class="group absolute bg-background flex gap-4 right-4 top-4 py-2 px-2 text-ink-muted">
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
