<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  Cell,
  Empty,
  List,
  NavBar,
  PullRefresh,
  Search,
  showToast,
  Tag as VanTag,
  Uploader,
  type UploaderFileListItem,
} from 'vant'
import { api, type FileRecord, type SearchResult } from '../api/client'
import { formatSize, formatTime, matchKindLabel, sourceLabel } from '../util/format'

const router = useRouter()

const query = ref('')
const searching = ref(false)
const searchResults = ref<SearchResult[] | null>(null)
const files = ref<FileRecord[]>([])
const refreshing = ref(false)

async function loadFiles() {
  try {
    files.value = await api.listFiles()
  } catch (e) {
    showToast(`加载失败：${(e as Error).message}`)
  }
}

async function onSearch() {
  const q = query.value.trim()
  if (!q) {
    searchResults.value = null
    return
  }
  searching.value = true
  try {
    searchResults.value = await api.searchDetailed(q)
  } catch (e) {
    showToast(`搜索失败：${(e as Error).message}`)
  } finally {
    searching.value = false
  }
}

function clearSearch() {
  query.value = ''
  searchResults.value = null
}

async function onRefresh() {
  if (searchResults.value !== null) {
    await onSearch()
  } else {
    await loadFiles()
  }
  refreshing.value = false
}

async function onUpload(items: UploaderFileListItem | UploaderFileListItem[]) {
  const list = Array.isArray(items) ? items : [items]
  for (const item of list) {
    if (!item.file) continue
    try {
      await api.uploadFile(item.file)
      showToast(`已导入 ${item.file.name}`)
    } catch (e) {
      showToast(`导入失败：${(e as Error).message}`)
    }
  }
  searchResults.value = null
  await loadFiles()
}

function openFile(id: string) {
  router.push(`/files/${encodeURIComponent(id)}`)
}

function matchHint(result: SearchResult): string {
  const kinds = [...new Set(result.matches.map((m) => matchKindLabel(m.match_kind)))]
  return `命中：${kinds.join('、')}`
}

onMounted(loadFiles)
</script>

<template>
  <NavBar title="deltabox">
    <template #right>
      <Uploader :after-read="onUpload">
        <span style="color: #1989fa">上传</span>
      </Uploader>
    </template>
  </NavBar>

  <Search
    v-model="query"
    placeholder="搜索文件名、标签或内容"
    :loading="searching"
    show-action
    @search="onSearch"
    @cancel="clearSearch"
  />

  <PullRefresh v-model="refreshing" @refresh="onRefresh">
    <!-- 搜索结果 -->
    <template v-if="searchResults !== null">
      <Empty v-if="searchResults.length === 0" description="没有匹配的文件" />
      <div
        v-for="result in searchResults"
        :key="result.file.file_id"
        class="search-result"
        @click="openFile(result.file.file_id)"
      >
        <Cell
          :title="result.file.name"
          :label="`${result.file.logical_path} · ${formatSize(result.file.size)}`"
        >
          <template #right-icon>
            <VanTag plain type="primary">{{ matchHint(result) }}</VanTag>
          </template>
        </Cell>
        <div
          v-for="(m, i) in result.matches.filter((m) => m.match_kind === 'text')"
          :key="i"
          class="match-snippet"
        >
          <VanTag type="success" plain>{{ sourceLabel(m.source) || '内容' }}</VanTag>
          <span v-if="m.page" class="locator">第 {{ m.page }} 页</span>
          <p>{{ m.text }}</p>
        </div>
      </div>
    </template>

    <!-- 文件列表 -->
    <template v-else>
      <Empty v-if="files.length === 0" description="还没有文件，点右上角上传" />
      <List>
        <Cell
          v-for="file in files"
          :key="file.file_id"
          :title="file.name"
          :label="`${file.logical_path} · ${formatSize(file.size)} · ${formatTime(file.imported_at)}`"
          is-link
          @click="openFile(file.file_id)"
        />
      </List>
    </template>
  </PullRefresh>
</template>

<style scoped>
.search-result {
  background: #fff;
  margin-bottom: 8px;
}
.match-snippet {
  padding: 4px 16px 10px;
  font-size: 13px;
  color: #646566;
}
.match-snippet p {
  margin: 4px 0 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.locator {
  margin-left: 6px;
  color: #969799;
}
</style>
