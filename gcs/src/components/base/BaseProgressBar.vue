<script lang="ts" setup>
const props = defineProps<{
  progress: number
}>()
</script>

<template>
  <div class="progress-bar">
    <div class="progress-fill" :style="`transform: scaleX(${props.progress.toString()})`"></div>
  </div>
</template>

<style scoped></style>
