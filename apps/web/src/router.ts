import { createRouter, createWebHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/files' },
    { path: '/files', component: () => import('./views/FileList.vue') },
    { path: '/files/:id', component: () => import('./views/FileDetail.vue') },
    { path: '/tags', component: () => import('./views/Tags.vue') },
    { path: '/index', component: () => import('./views/IndexJobs.vue') },
  ],
})
