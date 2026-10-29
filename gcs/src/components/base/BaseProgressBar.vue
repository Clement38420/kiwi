<script lang="ts" setup>
const props = defineProps<{
  progress: number
  color?: string
}>()
</script>

<template>
  <div class="progress-bar">
    <div :class="['progress-fill', color]" :style="`transform: scaleX(${props.progress})`"></div>
  </div>
</template>

<style scoped></style>
