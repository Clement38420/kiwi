<script setup lang="ts">
import BrandLogo from '@/components/brand/BrandLogo.vue'

const { class: className = 'text-xl' } = defineProps<{
  class?: string
}>()
</script>

<template>
  <div :class="['brand', className]">
    <BrandLogo />
    <span> Kiwi <b class="brand-accent">GCS</b> </span>
  </div>
</template>

<style scoped></style>
