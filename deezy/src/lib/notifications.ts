import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { invoke } from '@tauri-apps/api/core';

class NotificationManager {
  private permissionGranted: boolean | null = null;
  private notificationsEnabled: boolean = true;
  private pendingNotifications: Array<{ title: string; body: string }> = [];

  async initialize() {
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
        this.flushPendingNotifications();
      }

      return this.permissionGranted;
    } catch (err) {
      console.error('Failed to check notification permission:', err);
      return false;
    }
  }

  setEnabled(enabled: boolean) {
    this.notificationsEnabled = enabled;
  }

  private async flushPendingNotifications() {
    if (this.pendingNotifications.length === 0) return;

    if (this.pendingNotifications.length === 1) {
      const notif = this.pendingNotifications[0];
      await this.sendNotificationInternal(notif.title, notif.body);
    } else {
      const count = this.pendingNotifications.length;
      await this.sendNotificationInternal(
        'Downloads Complete',
        `${count} tracks have finished downloading`
      );
    }

    this.pendingNotifications = [];
  }

  private async sendNotificationInternal(title: string, body: string) {
    try {
      await sendNotification({ title, body });
    } catch (err) {
      console.error('Failed to send notification:', err);
    }
  }

  async notifyDownloadComplete(title: string, artist: string) {
    if (!this.notificationsEnabled) return;

    const notificationTitle = 'Download Complete';
    const notificationBody = `${title} - ${artist}`;

    if (this.permissionGranted === null) {
      await this.checkPermission();
    }

    if (this.permissionGranted) {
      await this.sendNotificationInternal(notificationTitle, notificationBody);
    } else {
      this.pendingNotifications.push({ 
        title: notificationTitle, 
        body: notificationBody 
      });
    }
  }

  async notifyDownloadError(title: string, artist: string, error: string) {
    if (!this.notificationsEnabled) return;

    const notificationTitle = 'Download Failed';
    const notificationBody = `${title} - ${artist}\n${error}`;

    if (this.permissionGranted === null) {
      await this.checkPermission();
    }

    if (this.permissionGranted) {
      await this.sendNotificationInternal(notificationTitle, notificationBody);
    } else {
      this.pendingNotifications.push({ 
        title: notificationTitle, 
        body: notificationBody 
      });
    }
  }

  async notifyBatchComplete(count: number) {
    if (!this.notificationsEnabled) return;

    const notificationTitle = 'Downloads Complete';
    const notificationBody = `${count} track${count > 1 ? 's' : ''} finished downloading`;

    if (this.permissionGranted === null) {
      await this.checkPermission();
    }

    if (this.permissionGranted) {
      await this.sendNotificationInternal(notificationTitle, notificationBody);
    }
  }
}

export const notificationManager = new NotificationManager();
