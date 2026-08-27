<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  Cell,
  Dialog,
  Empty,
  Field,
  NavBar,
  PullRefresh,
  showConfirmDialog,
  showToast,
} from 'vant'
import { api, type TagRecord } from '../api/client'
import { formatTime } from '../util/format'

const tags = ref<TagRecord[]>([])
const refreshing = ref(false)
const showCreate = ref(false)
const createName = ref('')
const createType = ref('')
const showRename = ref(false)
const renameFrom = ref('')
const renameTo = ref('')

async function load() {
  try {
    tags.value = await api.listTags()
  } catch (e) {
    showToast(`加载失败：${(e as Error).message}`)
  }
}

async function onRefresh() {
  await load()
  refreshing.value = false
}

async function create() {
  const name = createName.value.trim()
  if (!name) return
  try {
    await api.createTag(name, createType.value.trim() || undefined)
    createName.value = ''
    createType.value = ''
    showCreate.value = false
    showToast('已创建')
    await load()
  } catch (e) {
    showToast(`创建失败：${(e as Error).message}`)
  }
}

function startRename(tag: TagRecord) {
  renameFrom.value = tag.name
  renameTo.value = tag.name
  showRename.value = true
}

async function rename() {
  const newName = renameTo.value.trim()
  if (!newName || newName === renameFrom.value) {
    showRename.value = false
    return
  }
  try {
    await api.renameTag(renameFrom.value, newName)
    showRename.value = false
    showToast('已重命名')
    await load()
  } catch (e) {
    showToast(`重命名失败：${(e as Error).message}`)
  }
}

async function remove(tag: TagRecord) {
  try {
    await showConfirmDialog({
      title: '删除标签',
      message: `确定删除标签「${tag.name}」吗？文件上的关联会一并移除。`,
    })
  } catch {
    return
  }
  try {
    await api.deleteTag(tag.name)
    showToast('已删除')
    await load()
  } catch (e) {
    showToast(`删除失败：${(e as Error).message}`)
  }
}

onMounted(load)
</script>

<template>
  <NavBar title="标签">
    <template #right>
      <span style="color: #1989fa" @click="showCreate = true">新建</span>
    </template>
  </NavBar>

  <PullRefresh v-model="refreshing" @refresh="onRefresh">
    <Empty v-if="tags.length === 0" description="还没有标签" />
    <Cell
      v-for="tag in tags"
      :key="tag.tag_id"
      :title="tag.name"
      :label="`${tag.tag_type} · ${formatTime(tag.created_at)}`"
    >
      <template #right-icon>
        <span class="action" @click="startRename(tag)">重命名</span>
        <span class="action danger" @click="remove(tag)">删除</span>
      </template>
    </Cell>
  </PullRefresh>

  <Dialog v-model:show="showCreate" title="新建标签" show-cancel-button @confirm="create">
    <Field v-model="createName" placeholder="标签名称" autofocus />
    <Field v-model="createType" placeholder="类型（可选，如 project / person）" />
  </Dialog>

  <Dialog v-model:show="showRename" title="重命名标签" show-cancel-button @confirm="rename">
    <Field v-model="renameTo" placeholder="新名称" autofocus />
  </Dialog>
</template>

<style scoped>
.action {
  color: #1989fa;
  margin-left: 12px;
}
.action.danger {
  color: #ee0a24;
}
</style>
