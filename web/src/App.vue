<template>
  <div class="shell">
    <aside class="side">
      <RouterLink class="brand" to="/">
        <span class="brand-text">
          <strong>文渊阁</strong>
          <small>WENYUAN COUNCIL</small>
        </span>
      </RouterLink>
      <nav>
        <RouterLink to="/">
          <PlusCircle :size="17" />
          <span>新建议题</span>
        </RouterLink>
        <RouterLink to="/history">
          <History :size="17" />
          <span>历史议题</span>
        </RouterLink>
        <RouterLink to="/prompt-library">
          <Library :size="17" />
          <span>提示词模板</span>
        </RouterLink>
        <RouterLink to="/config">
          <Settings :size="17" />
          <span>配置状态</span>
        </RouterLink>
      </nav>
    </aside>
    <main class="main">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { History, Library, PlusCircle, Settings } from '@lucide/vue'
</script>
