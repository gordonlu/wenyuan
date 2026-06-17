import { createRouter, createWebHistory } from 'vue-router'
import NewSessionView from './views/NewSessionView.vue'
import SessionWorkspaceView from './views/SessionWorkspaceView.vue'
import HistoryView from './views/HistoryView.vue'
import ConfigView from './views/ConfigView.vue'

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: NewSessionView },
    { path: '/sessions/:id', component: SessionWorkspaceView },
    { path: '/history', component: HistoryView },
    { path: '/config', component: ConfigView },
  ],
})
