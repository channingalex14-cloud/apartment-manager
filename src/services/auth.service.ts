/**
 * 认证服务
 * Phase 4: 权限系统基础
 */

import { callCommand } from "./api";

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  success: boolean;
  token?: string;
  user?: UserInfo;
  message?: string;
}

export interface UserInfo {
  id: number;
  username: string;
  role: string;
  displayName?: string;
}

export interface PermissionCheck {
  resource: string;
  permission: string;
  action: "read" | "write" | "manage" | "none";
}

export interface User {
  id: number;
  username: string;
  passwordHash: string;
  role: string;
  displayName: string | null;
  isActive: boolean;
}

export interface UserManagementResponse {
  success: boolean;
  message: string;
  userId: number | null;
}

export interface CreateUserRequest {
  username: string;
  password: string;
  role: string;
  displayName?: string;
}

export interface UpdateUserRequest {
  role?: string;
  displayName?: string;
  isActive?: boolean;
}

// 内存中的 token 管理
let currentToken: string | null = null;
let currentUser: UserInfo | null = null;

export const authService = {
  /** 登录 */
  async login(request: LoginRequest): Promise<LoginResponse> {
    const response = await callCommand<LoginResponse>("login", { request });
    if (response.success && response.token && response.user) {
      currentToken = response.token;
      currentUser = response.user;
      // 持久化到 localStorage
      localStorage.setItem("auth_token", response.token);
      localStorage.setItem("auth_user", JSON.stringify(response.user));
    }
    return response;
  },

  /** 登出 */
  async logout(): Promise<boolean> {
    if (currentToken) {
      await callCommand<boolean>("logout", { token: currentToken });
    }
    currentToken = null;
    currentUser = null;
    localStorage.removeItem("auth_token");
    localStorage.removeItem("auth_user");
    return true;
  },

  /** 获取当前用户信息 */
  async getCurrentUser(): Promise<UserInfo | null> {
    if (!currentToken) {
      // 从 localStorage 恢复
      const savedToken = localStorage.getItem("auth_token");
      const savedUser = localStorage.getItem("auth_user");
      if (savedToken && savedUser) {
        currentToken = savedToken;
        currentUser = JSON.parse(savedUser);
        return currentUser;
      }
      return null;
    }
    const response = await callCommand<UserInfo | null>("get_current_user", { token: currentToken });
    return response;
  },

  /** 检查权限 */
  async checkPermission(resource: string, permission: string, action: string): Promise<boolean> {
    if (!currentToken) return false;
    return callCommand<boolean>("check_permission", {
      token: currentToken,
      resource,
      permission,
      action,
    });
  },

  /** 获取当前 token */
  getToken(): string | null {
    return currentToken;
  },

  /** 获取当前用户 */
  getUser(): UserInfo | null {
    return currentUser;
  },

  /** 是否已登录 */
  isLoggedIn(): boolean {
    return currentToken !== null;
  },

  /** 恢复登录状态（页面刷新后调用） */
  restoreSession(): boolean {
    const savedToken = localStorage.getItem("auth_token");
    const savedUser = localStorage.getItem("auth_user");
    if (savedToken && savedUser) {
      currentToken = savedToken;
      currentUser = JSON.parse(savedUser);
      return true;
    }
    return false;
  },

  /** 是否为管理员 */
  isAdmin(): boolean {
    return currentUser?.role === "admin";
  },

  /** 是否为财务角色 */
  isFinance(): boolean {
    return currentUser?.role === "finance";
  },

  /** 是否有指定角色 */
  hasRole(role: string): boolean {
    return currentUser?.role === role;
  },

  async listUsers(): Promise<User[]> {
    return callCommand<User[]>("list_users");
  },

  async createUser(req: CreateUserRequest): Promise<UserManagementResponse> {
    return callCommand<UserManagementResponse>("create_user", { req });
  },

  async updateUser(id: number, req: UpdateUserRequest): Promise<UserManagementResponse> {
    return callCommand<UserManagementResponse>("update_user", { id, req });
  },

  async resetPassword(id: number, newPassword: string): Promise<UserManagementResponse> {
    return callCommand<UserManagementResponse>("reset_password", { id, newPassword });
  },

  async deleteUser(id: number): Promise<UserManagementResponse> {
    return callCommand<UserManagementResponse>("delete_user", { id });
  },
};
