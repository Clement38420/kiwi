<script setup lang="ts">
import type { BatteryStatus } from '@/bindings/BatteryStatus.ts'
import type { State } from '@/types.ts'
import { computed } from 'vue'
import { useRobotStore } from '@/stores/robot.ts'
import BaseProgressBar from '@/components/base/BaseProgressBar.vue'

const robotStore = useRobotStore()

const BATTERY_MIN_VOLTAGE = 17.4
const BATTERY_MAX_VOLTAGE = 21.5

interface Vital {
  label: string
  value: string
  state?: State
  living?: boolean
  dot?: boolean
  gauge?: number
}

const batteryState: Record<BatteryStatus, State> = {
  Critical: 'crit',
  Low: 'crit',
  Medium: 'warn',
  High: 'ok',
}

const clamp01 = (n: number) => Math.max(0, Math.min(1, n))

const connectionVital = computed<Vital>(() => {
  const label = 'Robot connection'
  switch (robotStore.connectionStatus) {
    case WebSocket.CONNECTING:
      return { label, value: 'Connecting...', state: 'warn', dot: true }
    case WebSocket.OPEN:
      return { label, value: 'Connected', state: 'ok', dot: true, living: true }
    case WebSocket.CLOSING:
      return { label, value: 'Disconnecting...', state: 'warn', dot: true }
    default:
      return { label, value: 'Disconnected', state: 'crit', dot: true }
  }
})

const batteryVital = computed<Vital>(() => ({
  label: 'Battery voltage',
  value: `${robotStore.batteryVoltage.toFixed(2)} V`,
  state: batteryState[robotStore.batteryStatus],
  gauge: clamp01(
    (robotStore.batteryVoltage - BATTERY_MIN_VOLTAGE) / (BATTERY_MAX_VOLTAGE - BATTERY_MIN_VOLTAGE),
  ),
}))

const vitals = computed(() => [connectionVital.value, batteryVital.value])
</script>

<template>
  <div class="group w-full md:w-1/2 xl:w-1/3 text-sm">
    <p class="group-title">Health</p>
    <div class="card w-full">
      <div class="tel-list">
        <div v-for="vital in vitals" :key="vital.label" class="tel-row">
          <div class="tel-header">
            <span class="tel-label">{{ vital.label }}</span>
            <span class="tel-value" :data-state="!vital.dot ? vital.state : undefined">
              <span
                v-if="vital.dot"
                class="status-dot"
                :data-state="vital.state"
                :data-living="vital.living || undefined"
              />
              <span>{{ vital.value }}</span>
            </span>
          </div>
          <BaseProgressBar
            v-if="vital.gauge !== undefined"
            :progress="vital.gauge"
            :data-state="vital.state"
          />
        </div>
      </div>
    </div>
  </div>
</template>
