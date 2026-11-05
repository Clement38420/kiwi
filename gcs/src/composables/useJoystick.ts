import { computed, onMounted, onUnmounted, ref, type Ref } from 'vue'

export function useJoystick(target: Ref<HTMLElement | null>, maxRadius = 60) {
  const isDragging = ref(false)
  const delta = ref({ x: 0, y: 0 })

  const normalized = computed(() => ({
    x: Number((delta.value.x / maxRadius).toFixed(2)),
    y: Number((delta.value.y / maxRadius).toFixed(2)),
  }))

  let startPos = { x: 0, y: 0 }

  function onPointerDown(e: PointerEvent) {
    if (!target.value) return
    isDragging.value = true
    startPos = { x: e.clientX, y: e.clientY }
    target.value.setPointerCapture(e.pointerId)
  }

  function onPointerMove(e: PointerEvent) {
    if (!isDragging.value) return

    const dx = e.clientX - startPos.x
    const dy = e.clientY - startPos.y

    const distance = Math.hypot(dx, dy)
    if (distance > maxRadius) {
      const angle = Math.atan2(dy, dx)
      delta.value = {
        x: Math.cos(angle) * maxRadius,
        y: Math.sin(angle) * maxRadius,
      }
    } else {
      delta.value = { x: dx, y: dy }
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (!isDragging.value || !target.value) return
    isDragging.value = false
    delta.value = { x: 0, y: 0 }
    target.value.releasePointerCapture(e.pointerId)
  }

  const style = computed(() => ({
    transform: `translate3d(${delta.value.x.toString()}px, ${delta.value.y.toString()}px, 0)`,
    transition: isDragging.value
      ? 'filter 0.15s ease-out'
      : 'transform 0.2s ease-out, filter 0.15s ease-out',
  }))

  onMounted(() => {
    const el = target.value
    if (!el) return
    el.addEventListener('pointerdown', onPointerDown)
    el.addEventListener('pointermove', onPointerMove)
    el.addEventListener('pointerup', onPointerUp)
    el.addEventListener('pointercancel', onPointerUp)
  })

  onUnmounted(() => {
    const el = target.value
    if (!el) return
    el.removeEventListener('pointerdown', onPointerDown)
    el.removeEventListener('pointermove', onPointerMove)
    el.removeEventListener('pointerup', onPointerUp)
    el.removeEventListener('pointercancel', onPointerUp)
  })

  return {
    delta,
    normalized,
    isDragging,
    style,
  }
}
