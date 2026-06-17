<template>
  <p v-if="message" class="error-state" role="alert">{{ message }}</p>
</template>

<script setup lang="ts">
defineProps<{ message?: string }>()
</script>
