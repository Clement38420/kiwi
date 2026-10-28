<script setup lang="ts">
const { class: className = 'size-8' } = defineProps<{
  class?: string
}>()
</script>

<template>
  <svg :class="['logo-mark', className]" aria-hidden="true">
    <use href="#kiwi-glyph" />
  </svg>
</template>

<style scoped></style>
