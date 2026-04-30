<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { authService } from "@/services/auth.service";

const router = useRouter();
const username = ref("");
const password = ref("");
const loading = ref(false);

async function handleLogin() {
  if (!username.value || !password.value) {
    ElMessage.warning("请输入用户名和密码");
    return;
  }

  loading.value = true;
  try {
    const response = await authService.login({
      username: username.value,
      password: password.value,
    });

    if (response.success) {
      ElMessage.success(`欢迎回来，${response.user?.displayName || response.user?.username}`);
      router.push("/dashboard");
    } else {
      ElMessage.error(response.message || "登录失败");
    }
  } catch (e) {
    ElMessage.error("登录失败，请检查网络连接");
    console.error(e);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="login-container">
    <div class="login-box">
      <div class="login-header">
        <h1>新逸公寓管理系统</h1>
        <p>V2.0.5</p>
      </div>
      <el-form class="login-form" @submit.prevent="handleLogin">
        <el-form-item>
          <el-input
            v-model="username"
            placeholder="用户名"
            prefix-icon="User"
            size="large"
            autocomplete="username"
          />
        </el-form-item>
        <el-form-item>
          <el-input
            v-model="password"
            type="password"
            placeholder="密码"
            prefix-icon="Lock"
            size="large"
            autocomplete="current-password"
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        <el-form-item>
          <el-button
            type="primary"
            size="large"
            :loading="loading"
            class="login-button"
            @click="handleLogin"
          >
            登录
          </el-button>
        </el-form-item>
      </el-form>
      <div class="login-footer">
        <p>请使用分配给您的账号登录</p>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.login-box {
  width: 400px;
  padding: 40px;
  background: white;
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.login-header {
  text-align: center;
  margin-bottom: 30px;

  h1 {
    margin: 0;
    font-size: 24px;
    color: #333;
    font-weight: 600;
  }

  p {
    margin: 8px 0 0;
    font-size: 12px;
    color: #999;
  }
}

.login-form {
  :deep(.el-form-item) {
    margin-bottom: 20px;
  }
}

.login-button {
  width: 100%;
}

.login-footer {
  text-align: center;
  margin-top: 20px;

  p {
    margin: 0;
    font-size: 12px;
    color: #999;
  }
}
</style>
