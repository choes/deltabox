<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Tabbar, TabbarItem } from 'vant'

const route = useRoute()
const router = useRouter()

const active = computed(() => {
  if (route.path.startsWith('/tags')) return 'tags'
  if (route.path.startsWith('/index')) return 'index'
  return 'files'
})

function onChange(name: string) {
  router.push(`/${name}`)
}
</script>

<template>
  <div class="app">
    <main class="content">
      <router-view />
    </main>
    <Tabbar :model-value="active" @change="onChange">
      <TabbarItem name="files" icon="description">文件</TabbarItem>
      <TabbarItem name="tags" icon="label-o">标签</TabbarItem>
      <TabbarItem name="index" icon="orders-o">任务</TabbarItem>
    </Tabbar>
  </div>
</template>

<style>
body {
  margin: 0;
  background: #f7f8fa;
}
.app {
  max-width: 720px;
  margin: 0 auto;
}
.content {
  padding-bottom: 60px;
}
</style>
