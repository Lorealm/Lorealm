import { createRouter, createWebHistory } from 'vue-router'
import { setupAuthGuards } from './guards'
import MainLayout from '@/layouts/MainLayout.vue'
import LoginPage from '@/views/auth/LoginView.vue'
import UserInfoView from '@/views/user/UserInfoView.vue'
import WriteView from '@/views/write/WriteView.vue'
import ContextView from '@/views/context/ContextView.vue'
import PlaceholderView from '@/views/PlaceholderView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      meta: { requiresAuth: true },
      children: [
        {
          path: 'user/me',
          name: 'me',
          component: UserInfoView,
        },
        {
          path: 'write',
          name: 'write',
          component: WriteView,
        },
        {
          path: 'context',
          name: 'context',
          component: ContextView,
        },
        {
          path: 'characters',
          name: 'characters',
          component: PlaceholderView,
        },
        {
          path: 'worlds',
          name: 'worlds',
          component: PlaceholderView,
        },
        {
          path: 'plugins',
          name: 'plugins',
          component: PlaceholderView,
        },
      ],
    },
    {
      path: '/login',
      name: 'login',
      component: LoginPage,
      meta: { requiresAuth: false },
    },
  ],
})

setupAuthGuards(router)

export default router
