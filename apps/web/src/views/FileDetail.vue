<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  ActionBar,
  ActionBarButton,
  Cell,
  CellGroup,
  Dialog,
  Field,
  NavBar,
  showConfirmDialog,
  showToast,
  Tag as VanTag,
} from 'vant'
import {
  api,
  type FileManifest,
  type TagRecord,
  type TextSegmentRecord,
} from '../api/client'
import { formatSize, formatTime, sourceLabel } from '../util/format'

const route = useRoute()
const router = useRouter()
const fileId = route.params.id as string

const manifest = ref<FileManifest | null>(null)
const tags = ref<TagRecord[]>([])
const segments = ref<TextSegmentRecord[]>([])
const showTagDialog = ref(false)
const newTagName = ref('')

async function load() {
  try {
    const [m, t, s] = await Promise.all([
      api.fileInfo(fileId),
      api.fileTags(fileId),
      api.fileSegments(fileId),
    ])
    manifest.value = m
    tags.value = t
    segments.value = s
  } catch (e) {
    showToast(`加载失败：${(e as Error).message}`)
  }
}

async function addTag() {
  const name = newTagName.value.trim()
  if (!name) return
  try {
    await api.attachTag(fileId, name)
    newTagName.value = ''
    showTagDialog.value = false
    tags.value = await api.fileTags(fileId)
    showToast('已添加标签')
  } catch (e) {
    showToast(`添加失败：${(e as Error).message}`)
  }
}

async function removeTag(name: string) {
  try {
    await api.detachTag(fileId, name)
    tags.value = await api.fileTags(fileId)
  } catch (e) {
    showToast(`移除失败：${(e as Error).message}`)
  }
}

async function confirmTrash() {
  try {
    await showConfirmDialog({
      title: '移入回收站',
      message: `确定要把「${manifest.value?.name}」移入回收站吗？`,
    })
  } catch {
    return
  }
  try {
    await api.trashFile(fileId)
    showToast('已移入回收站')
    router.back()
  } catch (e) {
    showToast(`操作失败：${(e as Error).message}`)
  }
}

function download() {
  window.open(api.downloadUrl(fileId), '_blank')
}

function segmentLocator(seg: TextSegmentRecord): string {
  if (seg.page) return `第 ${seg.page} 页`
  if (seg.start_ms != null) return `${Math.round(seg.start_ms / 1000)}s`
  if (seg.line_start != null) return `第 ${seg.line_start} 行`
  return ''
}

onMounted(load)
</script>

<template>
  <NavBar title="文件详情" left-arrow @click-left="router.back()" />

  <template v-if="manifest">
    <CellGroup inset title="基本信息">
      <Cell title="文件名" :value="manifest.name" />
      <Cell title="路径" :value="manifest.logical_path" />
      <Cell title="类型" :value="manifest.mime" />
      <Cell title="大小" :value="formatSize(manifest.size)" />
      <Cell title="导入时间" :value="formatTime(manifest.imported_at)" />
      <Cell title="Hash" :label="manifest.content_hash" />
    </CellGroup>

    <CellGroup inset title="标签">
      <div class="tag-row">
        <VanTag
          v-for="tag in tags"
          :key="tag.tag_id"
          size="medium"
          type="primary"
          closeable
          @close="removeTag(tag.name)"
        >
          {{ tag.name }}
        </VanTag>
        <VanTag size="medium" plain type="primary" @click="showTagDialog = true">
          + 添加
        </VanTag>
      </div>
    </CellGroup>

    <CellGroup v-if="segments.length" inset :title="`已索引内容（${segments.length} 段）`">
      <div v-for="seg in segments" :key="seg.segment_id" class="segment">
        <div class="segment-meta">
          <VanTag plain>{{ sourceLabel(seg.source) || seg.source }}</VanTag>
          <span class="locator">{{ segmentLocator(seg) }}</span>
        </div>
        <p>{{ seg.text }}</p>
      </div>
    </CellGroup>

    <ActionBar>
      <ActionBarButton type="primary" text="下载" @click="download" />
      <ActionBarButton type="danger" text="移入回收站" @click="confirmTrash" />
    </ActionBar>

    <Dialog
      v-model:show="showTagDialog"
      title="添加标签"
      show-cancel-button
      @confirm="addTag"
    >
      <Field v-model="newTagName" placeholder="标签名称" autofocus />
    </Dialog>
  </template>
</template>

<style scoped>
.tag-row {
  padding: 12px 16px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.segment {
  padding: 8px 16px;
  border-bottom: 1px solid #f2f3f5;
}
.segment-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}
.locator {
  font-size: 12px;
  color: #969799;
}
.segment p {
  margin: 4px 0 0;
  font-size: 13px;
  color: #323233;
  white-space: pre-wrap;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>
