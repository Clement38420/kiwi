<script setup lang="ts">
import AppBar from '@/components/app/AppBar.vue'
import ModuleHealth from '@/components/modules/ModuleHealth.vue'
import ModuleLidarMap from '@/components/modules/ModuleLidarMap.vue'
</script>

<template>
  <div class="bg-bg h-dvh flex flex-col">
    <AppBar />
    <main class="p-4 grid grid-cols-5 grid-rows-5 flex-1 gap-4 min-h-0">
      <ModuleHealth class="col-span-2 row-span-2" />
      <ModuleLidarMap class="col-span-3 row-span-3" />
    </main>
  </div>
</template>

<style scoped></style>
