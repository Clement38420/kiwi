<script setup lang="ts">
import AppBar from '@/components/app/AppBar.vue'
import ModuleHealth from '@/components/modules/ModuleHealth.vue'
import ModuleLidarMap from '@/components/modules/ModuleLidarMap.vue'
import ModuleControl from '@/components/modules/ModuleControl.vue'
</script>

<template>
  <div class="bg-background h-dvh flex flex-col">
    <AppBar />
    <main class="dashboard-grid p-4 grid flex-1 gap-4 min-h-0 overflow-y-auto">
      <ModuleHealth class="area-health" />
      <ModuleLidarMap class="area-lidar" />
      <ModuleControl class="area-control" />
    </main>
  </div>
</template>

<style scoped>
.dashboard-grid {
  grid-template-columns: 1fr;
  grid-auto-rows: minmax(150px, auto);
  grid-template-areas:
    'health'
    'health'
    'health'
    'control'
    'control'
    'lidar'
    'lidar'
    'lidar';
}

@media (min-width: 768px) {
  .dashboard-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    grid-template-rows: repeat(5, minmax(0, 1fr));
    grid-template-areas:
      'health  health  lidar lidar'
      'health  health  lidar lidar'
      'control control   .     .  '
      'control control   .     .  '
      '   .       .      .     .    ';
  }
}

@media (min-width: 1280px) {
  .dashboard-grid {
    grid-template-columns: repeat(5, minmax(0, 1fr));
    grid-template-areas:
      'health  health  lidar lidar lidar'
      'health  health  lidar lidar lidar'
      'control control lidar lidar lidar'
      'control control lidar lidar lidar'
      '   .      .       .     .     .  ';
  }
}

.area-health {
  grid-area: health;
}

.area-lidar {
  grid-area: lidar;
}

.area-control {
  grid-area: control;
}
</style>
