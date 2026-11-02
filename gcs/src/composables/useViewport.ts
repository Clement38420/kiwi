import { computed, onMounted, onUnmounted, ref, type ShallowRef, watchEffect } from 'vue'

export interface ViewportOptions {
  /** Pixels per world unit at zoom = 1 (default: 40) */
  pixelsPerUnit?: number
  /** Grid divisions per world unit at scale 0 (default: 10) */
  divsPerUnit?: number
  /** Minimum size of a grid division in pixels before subdividing (default: 8) */
  minDivSize?: number
  /** Subdivision factor between grid levels (default: 10) */
  subdivisionBase?: number
  /** Wheel zoom sensitivity divisor (default: 100) */
  zoomCoef?: number
}

export function useViewport(
  canvas: Readonly<ShallowRef<HTMLCanvasElement | null>>,
  onDraw?: (ctx: CanvasRenderingContext2D) => void,
  options: ViewportOptions = {},
) {
  const {
    pixelsPerUnit = 40,
    divsPerUnit = 10,
    minDivSize = 8,
    subdivisionBase = 10,
    zoomCoef = 100,
  } = options

  const canvasSize = ref({ w: 0, h: 0 })
  const zoom = ref(1)
  const pan = ref({ x: 0, y: 0 })
  const themeVersion = ref(0)
  let isPanning = false
  let mousePrevPos = { x: 0, y: 0 }

  /** Current pixels per world unit */
  const ppu = computed(() => pixelsPerUnit * zoom.value)

  /** Current grid divisions per world unit */
  const dpu = computed(() => {
    const scaleOrder = Math.floor(
      Math.log(ppu.value / minDivSize / divsPerUnit) / Math.log(subdivisionBase),
    )
    return divsPerUnit * Math.pow(subdivisionBase, scaleOrder)
  })

  /** Current world units per grid division (formatted to 1 significant digit) */
  const upd = computed(() => {
    const u = 1 / dpu.value
    if (Math.abs(Math.log10(u)) <= 3) {
      return Number(u.toPrecision(1)).toString()
    }
    return u.toExponential(0)
  })

  const gridSize = computed(() => {
    const divSizePx = ppu.value / dpu.value
    return `${divSizePx.toString()}px ${divSizePx.toString()}px`
  })

  const gridPosition = computed(() => {
    const x = canvasSize.value.w / 2 + pan.value.x
    const y = canvasSize.value.h / 2 + pan.value.y
    return `${x.toString()}px ${y.toString()}px`
  })

  watchEffect(() => {
    const el = canvas.value
    if (!el) return

    el.style.backgroundPosition = gridPosition.value
    el.style.backgroundSize = gridSize.value

    if (!onDraw) return
    const ctx = el.getContext('2d')
    if (!ctx) return

    // Track theme changes and canvas dimensions
    // eslint-disable-next-line @typescript-eslint/no-meaningless-void-operator
    void themeVersion.value
    const { w, h } = canvasSize.value

    ctx.resetTransform()
    ctx.clearRect(0, 0, w, h)
    ctx.translate(w / 2 + pan.value.x, h / 2 + pan.value.y)

    onDraw(ctx)
  })

  function onWheel(e: WheelEvent) {
    e.preventDefault()
    const factor = Math.exp(-e.deltaY / zoomCoef)
    const dx = e.offsetX - canvasSize.value.w / 2
    const dy = e.offsetY - canvasSize.value.h / 2

    zoom.value *= factor
    pan.value = {
      x: dx - (dx - pan.value.x) * factor,
      y: dy - (dy - pan.value.y) * factor,
    }
  }

  function onMouseDown(e: MouseEvent) {
    mousePrevPos = { x: e.x, y: e.y }
    isPanning = true
  }

  function onMouseUp() {
    isPanning = false
  }

  function onMouseMove(e: MouseEvent) {
    if (isPanning) {
      pan.value = {
        x: pan.value.x + e.x - mousePrevPos.x,
        y: pan.value.y + e.y - mousePrevPos.y,
      }

      mousePrevPos = { x: e.x, y: e.y }
    }
  }

  function resetViewport() {
    zoom.value = 1
    pan.value = { x: 0, y: 0 }
  }

  let resizeObserver: ResizeObserver | null = null
  let themeObserver: MutationObserver | null = null

  onMounted(() => {
    const el = canvas.value
    if (!el) return

    el.addEventListener('wheel', onWheel, { passive: false })
    el.addEventListener('mousedown', onMouseDown)
    el.addEventListener('mouseup', onMouseUp)
    el.addEventListener('mousemove', onMouseMove)
    el.addEventListener('mouseleave', onMouseUp)

    resizeObserver = new ResizeObserver(([entry]) => {
      if (!entry) return

      el.height = entry.contentRect.height
      el.width = entry.contentRect.width
      canvasSize.value = { w: entry.contentRect.width, h: entry.contentRect.height }
    })

    resizeObserver.observe(el)

    themeObserver = new MutationObserver(() => {
      themeVersion.value++
    })
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })
  })

  onUnmounted(() => {
    resizeObserver?.disconnect()
    themeObserver?.disconnect()
    const el = canvas.value
    if (!el) return
    el.removeEventListener('wheel', onWheel)
    el.removeEventListener('mousedown', onMouseDown)
    el.removeEventListener('mouseup', onMouseUp)
    el.removeEventListener('mousemove', onMouseMove)
    el.removeEventListener('mouseleave', onMouseUp)
  })

  return {
    canvasSize,
    zoom,
    pan,
    ppu,
    dpu,
    upd,
    resetViewport,
  }
}
