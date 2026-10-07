export type UpdatePhase = 'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'ready' | 'installing' | 'restart' | 'error' | 'preview';
export interface UpdateStatus { phase: UpdatePhase; message: string; version?: string; received?: number; total?: number }
export type DownloadEvent = { event: 'Started'; data: { contentLength?: number } } | { event: 'Progress'; data: { chunkLength: number } } | { event: 'Finished' };
export interface PackageUpdate {
  version: string;
  download: (progress: (event: DownloadEvent) => void, options?: { timeout: number }) => Promise<void>;
  install: (options?: { restartAfterInstall: boolean }) => Promise<void>;
  close: () => Promise<void>;
}

/** One in-memory verified package. A download never installs or restarts the app. */
export class UpdateController {
  private update: PackageUpdate | null = null;
  private busy = false;
  private disposed = false;
  private downloaded = false;
  get canDownload(): boolean { return !!this.update && !this.downloaded && !this.busy; }
  get canInstall(): boolean { return !!this.update && this.downloaded && !this.busy; }
  private async finish(): Promise<void> {
    this.busy = false;
    if (this.disposed) { await this.update?.close(); this.update = null; }
  }
  constructor(private checkPackage: () => Promise<PackageUpdate | null>, private publish: (status: UpdateStatus) => void) {}
  async check(autoDownload = false): Promise<void> {
    if (this.busy || this.disposed || this.downloaded) return;
    this.busy = true;
    this.publish({ phase: 'checking', message: 'Checking GitHub releases…' });
    try {
      await this.update?.close(); this.update = null;
      const candidate = await this.checkPackage();
      if (this.disposed) { await candidate?.close(); return; }
      this.update = candidate;
      this.publish(candidate ? { phase: 'available', version: candidate.version, message: `Version ${candidate.version} is available.` } : { phase: 'current', message: 'You have the latest published version.' });
    } catch { this.publish({ phase: 'error', message: 'Could not check GitHub releases. Check your connection and retry.' }); }
    finally { await this.finish(); }
    if (autoDownload && this.update && !this.disposed) await this.download();
  }
  async download(): Promise<void> {
    if (this.busy || !this.update || this.disposed || this.downloaded) return;
    this.busy = true;
    let received = 0, total: number | undefined;
    const version = this.update.version;
    const progress = () => this.publish({ phase: 'downloading', version, received, total, message: 'Downloading update… signature verification follows.' });
    progress();
    try {
      await this.update.download(event => {
        if (event.event === 'Started') { total = event.data.contentLength; received = 0; }
        if (event.event === 'Progress') received += event.data.chunkLength;
        if (!this.disposed) progress();
      }, { timeout: 120000 });
      if (this.disposed) return;
      this.downloaded = true;
      this.publish({ phase: 'ready', version, message: `Version ${version} downloaded and signature verified. Install when you are ready.` });
    } catch { this.publish({ phase: 'error', version, message: 'Update download or signature verification failed. The installed app has not changed; retry the download.' }); }
    finally { await this.finish(); }
  }
  /** Called only by the user's Install & restart button. Windows may exit before resolving. */
  async install(): Promise<void> {
    if (this.busy || !this.update || !this.downloaded || this.disposed) return;
    this.busy = true;
    this.publish({ phase: 'installing', version: this.update.version, message: 'Installing update… Windows will close this app and restart after setup.' });
    try {
      await this.update.install({ restartAfterInstall: true });
      this.publish({ phase: 'restart', version: this.update.version, message: 'Update installed. Restart Neon HUD to load it.' });
    } catch { this.publish({ phase: 'error', version: this.update.version, message: 'Update installation failed. Retry installation or use the installer from GitHub.' }); }
    finally { await this.finish(); }
  }
  async dispose(): Promise<void> { this.disposed = true; if (!this.busy) { await this.update?.close(); this.update = null; } }
}
