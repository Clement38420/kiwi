<script setup lang="ts">
import { useTemplateRef, watch } from 'vue'
import { useKiwiStore } from '@/stores/kiwi.ts'
import type { LidarPoint } from '@/bindings/LidarPoint.ts'

const kiwiStore = useKiwiStore()

const canvasRef = useTemplateRef<HTMLCanvasElement>('canvas')

watch(
  () => kiwiStore.lidarDataPoints,
  (newPoints) => {
    drawScan(newPoints)
  },
)

function drawScan(points: LidarPoint[]) {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const cx = canvas.width / 2
  const cy = canvas.height / 2

  ctx.clearRect(0, 0, canvas.width, canvas.height)

  // Robot center
  ctx.fillStyle = '#ef4444'
  ctx.beginPath()
  ctx.arc(cx, cy, 4, 0, Math.PI * 2)
  ctx.fill()

  // LiDAR points
  ctx.fillStyle = '#84cc16'
  for (const pt of points) {
    if (pt.distance <= 0) continue
    const rad = (pt.angle * Math.PI) / 180
    const y = cx - (Math.cos(rad) * pt.distance) / 10
    const x = cy + (Math.sin(rad) * pt.distance) / 10
    ctx.fillRect(x - 1.5, y - 1.5, 3, 3)
  }
}
</script>

<template>
  <main>
    <h1 class="text-2xl">Kiwi Ground Control Station</h1>
    <canvas ref="canvas" class="border-black border-2" width="1000" height="1000"></canvas>
  </main>
</template>

<style>
@import 'tailwindcss';
</style>
