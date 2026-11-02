<script setup lang="ts">
import BrandLogotype from '@/components/brand/BrandLogotype.vue'
import { Moon, Sun } from '@lucide/vue'
import { ref, watchEffect } from 'vue'

type Theme = 'light' | 'dark'

const theme = ref<Theme>(
  (localStorage.getItem('theme') as Theme | null) ??
    (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'),
)

watchEffect(() => {
  document.documentElement.dataset.theme = theme.value
  localStorage.setItem('theme', theme.value)
})

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
}
</script>

<template>
  <div class="border-b h-15 p-4 flex items-center justify-between">
    <BrandLogotype class="text-xl" />
    <div>
      <button class="btn btn-icon" @click="toggleTheme">
        <Sun v-if="theme === 'dark'" />
        <Moon v-else />
      </button>
    </div>
  </div>
</template>
