<template>
  <div class="shell">
    <aside class="side">
      <RouterLink class="brand" to="/">
        <span class="brand-mark">文</span>
        <span>
          <strong>文渊阁</strong>
          <small>三席合议</small>
        </span>
      </RouterLink>
      <nav>
        <RouterLink to="/">新建议题</RouterLink>
        <RouterLink to="/history">历史议题</RouterLink>
        <RouterLink to="/config">配置状态</RouterLink>
      </nav>
    </aside>
    <main class="main">
      <RouterView />
    </main>
  </div>
</template>
