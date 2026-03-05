import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { invoke } from '@tauri-apps/api/core';

interface PendingNotification {
  title: string;
  body: string;
}

const MAX_PENDING_NOTIFICATIONS = 10;
const BATCH_NOTIFICATION_THRESHOLD = 2;

class NotificationManager {
  private permissionGranted: boolean | null = null;
  private notificationsEnabled = true;
  private pendingNotifications: PendingNotification[] = [];

  async initialize(): Promise<void> {
    try {
      const settings: any = await invoke('get_settings');
      this.notificationsEnabled = settings.notifications_enabled ?? true;
    } catch (err) {
      console.error('Failed to load notification settings:', err);
      this.notificationsEnabled = true;
    }

    await this.checkPermission();
  }

  async checkPermission(): Promise<boolean> {
    try {
      this.permissionGranted = await isPermissionGranted();
      
      if (!this.permissionGranted) {
        const permission = await requestPermission();
        this.permissionGranted = permission === 'granted';
      }

      if (this.permissionGranted && this.pendingNotifications.length > 0) {
        await this.flushPendingNotifications();
      }

      return this.permissionGranted;
    } catch (err) {
      console.error('Failed to check notification permission:', err);
      return false;
    }
  }

  setEnabled(enabled: boolean): void {
    this.notificationsEnabled = enabled;
  }

  getEnabled(): boolean {
    return this.notificationsEnabled;
  }

  private async flushPendingNotifications(): Promise<void> {
    if (this.pendingNotifications.length === 0) return;

    try {
      if (this.pendingNotifications.length < BATCH_NOTIFICATION_THRESHOLD) {
        const notif = this.pendingNotifications[0];
        await this.sendNotificationInternal(notif.title, notif.body);
      } else {
        const count = this.pendingNotifications.length;
        await this.sendNotificationInternal(
          'Downloads Complete',
          `${count} tracks have finished downloading`
        );
      }
    } finally {
      this.pendingNotifications = [];
    }
  }

  private async sendNotificationInternal(title: string, body: string): Promise<void> {
    try {
      await sendNotification({ title, body });
    } catch (err) {
      console.error('Failed to send notification:', err);
    }
  }

  private addToPending(title: string, body: string): void {
    if (this.pendingNotifications.length >= MAX_PENDING_NOTIFICATIONS) {
      this.pendingNotifications.shift();
    }
    this.pendingNotifications.push({ title, body });
  }

  private async ensurePermission(): Promise<boolean> {
    if (this.permissionGranted === null) {
      await this.checkPermission();
    }
    return this.permissionGranted ?? false;
  }

  private async sendOrQueue(title: string, body: string): Promise<void> {
    if (await this.ensurePermission()) {
      await this.sendNotificationInternal(title, body);
    } else {
      this.addToPending(title, body);
    }
  }

  async notifyDownloadComplete(title: string, artist: string): Promise<void> {
    if (!this.notificationsEnabled) return;

    const notificationTitle = 'Download Complete';
    const notificationBody = `${title} - ${artist}`;

    await this.sendOrQueue(notificationTitle, notificationBody);
  }

  async notifyDownloadError(title: string, artist: string, error: string): Promise<void> {
    if (!this.notificationsEnabled) return;

    const notificationTitle = 'Download Failed';
    const truncatedError = error.length > 100 ? `${error.substring(0, 100)}...` : error;
    const notificationBody = `${title} - ${artist}\n${truncatedError}`;

    await this.sendOrQueue(notificationTitle, notificationBody);
  }

  async notifyBatchComplete(count: number): Promise<void> {
    if (!this.notificationsEnabled || count <= 0) return;

    const notificationTitle = 'Downloads Complete';
    const plural = count > 1 ? 's' : '';
    const notificationBody = `${count} track${plural} finished downloading`;

    await this.sendOrQueue(notificationTitle, notificationBody);
  }

  clearPending(): void {
    this.pendingNotifications = [];
  }

  getPendingCount(): number {
    return this.pendingNotifications.length;
  }
}

export const notificationManager = new NotificationManager();
