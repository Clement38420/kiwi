<script setup lang="ts">
import type { BatteryLevel } from '@/bindings/BatteryLevel.ts'
import type { State } from '@/utils/types.ts'
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

const batteryState: Record<BatteryLevel, State> = {
  Critical: 'crit',
  Low: 'crit',
  Medium: 'warn',
  High: 'ok',
}

const clamp01 = (n: number) => Math.max(0, Math.min(1, n))

const connectionVital = computed<Vital>(() => {
  const label = 'Robot connection'
  switch (robotStore.connectionState) {
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
  value: `${robotStore.robotState.battery_voltage.toFixed(2)} V`,
  state: batteryState[robotStore.robotState.battery_level],
  gauge: clamp01(
    (robotStore.robotState.battery_voltage - BATTERY_MIN_VOLTAGE) /
      (BATTERY_MAX_VOLTAGE - BATTERY_MIN_VOLTAGE),
  ),
}))

const vitals = computed(() => [connectionVital.value, batteryVital.value])
</script>

<template>
  <div class="group text-sm flex flex-col gap-4 w-full">
    <p class="group-title">Health</p>
    <div class="card w-full">
      <div class="tel-list">
        <div v-for="vital in vitals" :key="vital.label" class="tel-row">
          <div class="tel-header">
            <span class="tel-label">{{ vital.label }}</span>
            <span class="tel-value" :data-state="!vital.dot ? vital.state : undefined">
              <span
                v-if="vital.dot"
                class="state-dot"
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
    <div class="grid grid-cols-4">
      <button
        class="btn btn-semantic"
        :data-state="robotStore.robotState.arm === 'Armed' ? 'crit' : 'ok'"
        :disabled="robotStore.robotState.arm === 'CannotArm' || !robotStore.isConnected"
        @click="robotStore.sendMessage({ Arm: robotStore.robotState.arm !== 'Armed' })"
      >
        {{ robotStore.robotState.arm === 'Armed' ? 'Disarm' : 'Arm' }}
      </button>
    </div>
  </div>
</template>
