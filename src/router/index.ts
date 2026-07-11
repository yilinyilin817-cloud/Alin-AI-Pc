import { createRouter, createWebHashHistory } from "vue-router";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    {
      path: "/dashboard",
      name: "dashboard",
      component: () => import("@/views/DashboardView.vue"),
    },
    {
      path: "/chat/:sessionId?",
      name: "chat",
      component: () => import("@/views/Chat.vue"),
    },
    {
      path: "/persona/:id?",
      name: "persona",
      component: () => import("@/views/PersonaEditor.vue"),
    },
    {
      path: "/knowledge",
      name: "knowledge",
      component: () => import("@/views/KnowledgeBase.vue"),
    },
    {
      path: "/skills",
      name: "skills",
      component: () => import("@/views/SkillMarket.vue"),
    },
    {
      path: "/models",
      name: "models",
      component: () => import("@/views/ModelCenter.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("@/views/Settings.vue"),
    },
    {
      path: "/plugins",
      name: "plugins",
      component: () => import("@/views/PluginManager.vue"),
    },
    {
      path: "/wechat/:sessionId?",
      name: "wechat",
      component: () => import("@/views/WeChatView.vue"),
    },
  ],
});

export default router;
