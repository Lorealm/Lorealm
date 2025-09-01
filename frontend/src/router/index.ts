import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue' // Import your new layout
import ChatView from '@/views/ChatView.vue'
import SettingsView from '@/views/SettingsView.vue'
import LoginPage from '@/pages/LoginPage.vue'
import UserInfoView from '@/views/user/UserInfoView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [ 
        {
          path: '', 
          name: 'home',
          component: ChatView,
        },
        {
          path: 'settings',
          name: 'settings',
          component: SettingsView,
        },
        {
          path: 'user/me',
          name: 'me',
          component: UserInfoView,
        }
      ],
    },
    {
      path: '/login',
      name: 'login',
      component: LoginPage,
    }
    // Add more top-level routes here for other full-page screens (e.g., signup, error pages)
  ],
})

export default router