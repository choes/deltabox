<script setup lang="ts">
import { onMounted, ref } from 'vue'
import {
  Button,
  Cell,
  Empty,
  NavBar,
  Progress,
  PullRefresh,
  showToast,
  Tag as VanTag,
} from 'vant'
import { api, type IndexJobRecord } from '../api/client'
import { formatTime, jobStatusLabel } from '../util/format'

const jobs = ref<IndexJobRecord[]>([])
const refreshing = ref(false)
const running = ref(false)

async function load() {
  try {
    jobs.value = await api.listIndexJobs()
  } catch (e) {
    showToast(`加载失败：${(e as Error).message}`)
  }
}

async function onRefresh() {
  await load()
  refreshing.value = false
}

async function act(job: IndexJobRecord, action: 'pause' | 'resume' | 'retry' | 'cancel') {
  try {
    await api.indexJobAction(job.job_id, action)
    await load()
  } catch (e) {
    showToast(`操作失败：${(e as Error).message}`)
  }
}

async function runWorker() {
  running.value = true
  try {
    const summary = await api.runIndexWorker(10)
    showToast(`完成 ${summary.completed}，失败 ${summary.failed}，跳过 ${summary.skipped}`)
    await load()
  } catch (e) {
    showToast(`运行失败：${(e as Error).message}`)
  } finally {
    running.value = false
  }
}

function progress(job: IndexJobRecord): number {
  if (job.total_tasks === 0) return job.status === 'completed' ? 100 : 0
  return Math.round((job.completed_tasks / job.total_tasks) * 100)
}

function statusType(status: string): 'success' | 'warning' | 'danger' | 'primary' {
  if (status === 'completed') return 'success'
  if (status === 'running' || status === 'pending') return 'primary'
  if (status === 'paused') return 'warning'
  return 'danger'
}

onMounted(load)
</script>

<template>
  <NavBar title="索引任务">
    <template #right>
      <Button size="mini" type="primary" :loading="running" @click="runWorker">
        运行 worker
      </Button>
    </template>
  </NavBar>

  <PullRefresh v-model="refreshing" @refresh="onRefresh">
    <Empty v-if="jobs.length === 0" description="没有索引任务" />
    <div v-for="job in jobs" :key="job.job_id" class="job-card">
      <Cell
        :title="job.job_type"
        :label="`${job.file_id.slice(0, 8)}… · ${formatTime(job.updated_at)}`"
      >
        <template #value>
          <VanTag :type="statusType(job.status)">{{ jobStatusLabel(job.status) }}</VanTag>
        </template>
      </Cell>
      <div class="job-body">
        <Progress :percentage="progress(job)" stroke-width="6" />
        <div class="job-stats">
          {{ job.completed_tasks }}/{{ job.total_tasks }} 段
          <span v-if="job.failed_tasks">· 失败 {{ job.failed_tasks }}</span>
        </div>
        <p v-if="job.last_error" class="job-error">{{ job.last_error }}</p>
        <div class="job-actions">
          <Button
            v-if="job.status === 'running' || job.status === 'pending'"
            size="small"
            @click="act(job, 'pause')"
          >
            暂停
          </Button>
          <Button
            v-if="job.status === 'paused'"
            size="small"
            type="primary"
            @click="act(job, 'resume')"
          >
            继续
          </Button>
          <Button
            v-if="job.status.startsWith('failed')"
            size="small"
            type="warning"
            @click="act(job, 'retry')"
          >
            重试
          </Button>
          <Button
            v-if="['pending', 'running', 'paused'].includes(job.status)"
            size="small"
            type="danger"
            @click="act(job, 'cancel')"
          >
            取消
          </Button>
        </div>
      </div>
    </div>
  </PullRefresh>
</template>

<style scoped>
.job-card {
  background: #fff;
  margin-bottom: 8px;
}
.job-body {
  padding: 4px 16px 12px;
}
.job-stats {
  margin-top: 4px;
  font-size: 12px;
  color: #969799;
}
.job-error {
  margin: 4px 0 0;
  font-size: 12px;
  color: #ee0a24;
}
.job-actions {
  margin-top: 8px;
  display: flex;
  gap: 8px;
}
</style>
