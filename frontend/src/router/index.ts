import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/layouts/MainLayout.vue' // Import your new layout
import LoginPage from '@/pages/LoginPage.vue'
import UserInfoView from '@/views/user/UserInfoView.vue'
import WriteView from '@/views/WriteView.vue'
import ContextView from '@/views/ContextView.vue'
import PlaceholderView from '@/views/PlaceholderView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: MainLayout,
      children: [ 
        {
          path: 'user/me',
          name: 'me',
          component: UserInfoView,
        },
        {
          path: 'write',
          name: 'write',
          component: WriteView
        },
        {
          path: 'context',
          name: 'context',
          component: ContextView
        },
        {
          path: 'characters',
          name: 'characters',
          component: PlaceholderView
        },
        {
          path: 'worlds',
          name: 'worlds',
          component: PlaceholderView
        }
        ,
        {
          path: 'plugins',
          name: 'plugins',
          component: PlaceholderView
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