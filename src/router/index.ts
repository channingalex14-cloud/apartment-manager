import { createRouter, createWebHashHistory } from "vue-router";
import { authService } from "@/services/auth.service";
import AppLayout from "@/components/common/AppLayout.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/login",
      name: "Login",
      component: () => import("@/views/Login.vue"),
      meta: { title: "登录", requiresAuth: false },
    },
    {
      path: "/",
      component: AppLayout,
      redirect: "/dashboard",
      meta: { requiresAuth: true },
      children: [
        {
          path: "/dashboard",
          name: "Dashboard",
          component: () => import("@/views/Dashboard.vue"),
          meta: { title: "首页", requiresAuth: true },
        },
        {
          path: "/rooms",
          name: "RoomList",
          component: () => import("@/views/RoomList.vue"),
          meta: { title: "房态管理", requiresAuth: true },
        },
        {
          path: "/tenants",
          name: "TenantList",
          component: () => import("@/views/TenantList.vue"),
          meta: { title: "租客管理", requiresAuth: true },
        },
        {
          path: "/leases",
          name: "LeaseList",
          component: () => import("@/views/LeaseList.vue"),
          meta: { title: "合同管理", requiresAuth: true },
        },
        {
          path: "/bills",
          name: "BillList",
          component: () => import("@/views/BillList.vue"),
          meta: { title: "账单管理", requiresAuth: true },
        },
        {
          path: "/bills/generate",
          name: "BillGenerate",
          component: () => import("@/views/BillGenerate.vue"),
          meta: { title: "生成账单", requiresAuth: true },
        },
        {
          path: "/bills/:id",
          name: "BillDetail",
          component: () => import("@/views/BillDetail.vue"),
          meta: { title: "账单详情", requiresAuth: true },
        },
        {
          path: "/bills/:id/print",
          name: "BillPrint",
          component: () => import("@/views/BillPrint.vue"),
          meta: { title: "打印收费通知单", requiresAuth: true },
        },
        {
          path: "/payments",
          name: "PaymentList",
          component: () => import("@/views/PaymentList.vue"),
          meta: { title: "缴费记录", requiresAuth: true },
        },
        {
          path: "/deposits",
          name: "DepositLedger",
          component: () => import("@/views/DepositLedger.vue"),
          meta: { title: "押金台账", requiresAuth: true, requiredRole: "admin" },
        },
        {
          path: "/reports",
          name: "Reports",
          component: () => import("@/views/Reports.vue"),
          meta: { title: "月度报表", requiresAuth: true },
        },
        {
          path: "/documents",
          name: "Documents",
          component: () => import("@/views/Documents.vue"),
          meta: { title: "文档管理", requiresAuth: true },
        },
        {
          path: "/users",
          name: "UserManagement",
          component: () => import("@/views/UserManagement.vue"),
          meta: { title: "用户管理", requiresAuth: true, requiredRole: "admin" },
        },
        {
          path: "/settings",
          name: "Settings",
          component: () => import("@/views/Settings.vue"),
          meta: { title: "系统配置", requiresAuth: true, requiredRole: "admin" },
        },
        {
          path: "/reminders",
          name: "Reminders",
          component: () => import("@/views/Reminders.vue"),
          meta: { title: "提醒管理", requiresAuth: true },
        },
      ],
    },
  ],
});

// 路由守卫
router.beforeEach(async (to, _from, next) => {
  const requiresAuth = to.matched.some((record) => record.meta.requiresAuth !== false);

  if (requiresAuth && !authService.isLoggedIn()) {
    next({ name: "Login", query: { redirect: to.fullPath } });
  } else if (to.name === "Login" && authService.isLoggedIn()) {
    next({ name: "Dashboard" });
  } else if (to.meta.requiredRole) {
    const user = authService.getUser();
    if (user && user.role === to.meta.requiredRole) {
      next();
    } else {
      next({ name: "Dashboard" });
    }
  } else {
    next();
  }
});

export default router;
