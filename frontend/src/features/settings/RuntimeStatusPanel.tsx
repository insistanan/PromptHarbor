import type { AppStatus } from '../../appTypes';

export function RuntimeStatusPanel({ status }: { status: AppStatus | null }) {
  return (
    <section className="runtime-panel" aria-label="本地运行时状态">
      <div className="section-heading">
        <h3>本地运行时</h3>
        <span>{status?.recordingPaused ? '记录暂停' : '记录中'}</span>
      </div>
      <dl className="runtime-list">
        <div>
          <dt>数据目录</dt>
          <dd>{status?.promptboxHome ?? '未初始化'}</dd>
        </div>
        <div>
          <dt>用户配置</dt>
          <dd>{status?.configPath ?? '未初始化'}</dd>
        </div>
        <div>
          <dt>钩子可执行文件</dt>
          <dd>{status?.hookBinaryPath ?? '未初始化'}</dd>
        </div>
        <div>
          <dt>钩子状态</dt>
          <dd className={status?.hookBinaryReady ? 'ok-text' : 'warning-text'}>
            {status?.hookBinaryMessage ?? '等待检测'}
          </dd>
        </div>
        <div>
          <dt>数据库</dt>
          <dd className={status?.databaseReady ? 'ok-text' : 'warning-text'}>
            {status?.databaseMessage ?? '等待初始化'}
          </dd>
        </div>
        <div>
          <dt>采集端点</dt>
          <dd className={status?.collectorReady ? 'ok-text' : 'warning-text'}>
            {status?.collectorMessage ?? '等待启动'}
          </dd>
        </div>
        <div>
          <dt>记录状态</dt>
          <dd className={status?.recordingPaused ? 'warning-text' : 'ok-text'}>
            {status?.recordingPaused ? '已暂停，不写入提示词' : '记录中'}
          </dd>
        </div>
        <div>
          <dt>智能体会话</dt>
          <dd>{status ? `${status.sessionCount} 个` : '0 个'}</dd>
        </div>
        <div>
          <dt>正式提示词</dt>
          <dd>{status ? `${status.promptEventCount} 条` : '0 条'}</dd>
        </div>
        <div>
          <dt>已采集事件</dt>
          <dd>{status ? `${status.receivedPromptEvents} 条` : '0 条'}</dd>
        </div>
        <div>
          <dt>暂停丢弃</dt>
          <dd>{status ? `${status.pausedPromptEvents} 条` : '0 条'}</dd>
        </div>
        <div>
          <dt>启动导入</dt>
          <dd>{status ? `${status.importedSpoolEvents} 条` : '0 条'}</dd>
        </div>
      </dl>
      {status?.startupErrors.length ? (
        <div className="runtime-errors">
          {status.startupErrors.map((item) => (
            <p key={item}>{item}</p>
          ))}
        </div>
      ) : null}
    </section>
  );
}
