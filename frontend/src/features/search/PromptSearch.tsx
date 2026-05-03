import { useEffect, useState } from 'react';
import * as api from '../../api';

export type PromptSearchResultItem = {
  provider: string;
  providerLabel: string;
  sessionId: string;
  shortSessionId: string;
  title: string;
  projectName: string;
  matchKind: string;
  matchLabel: string;
  snippet: string;
  isLowInfo: boolean;
  sentAt: string | null;
  updatedAt: string;
};

type PromptSearchResults = {
  query: string;
  items: PromptSearchResultItem[];
};

export function PromptSearch({
  hideLowInfo,
  onHideLowInfoChange,
  onResultCountChange,
  onSelect,
}: {
  hideLowInfo: boolean;
  onHideLowInfoChange: (value: boolean) => void;
  onResultCountChange: (count: number) => void;
  onSelect: (item: PromptSearchResultItem) => void;
}) {
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<PromptSearchResults>({
    query: '',
    items: [],
  });
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchError, setSearchError] = useState<string | null>(null);
  const includeLowInfo = !hideLowInfo;

  useEffect(() => {
    onResultCountChange(searchResults.items.length);
  }, [onResultCountChange, searchResults.items.length]);

  useEffect(() => {
    let disposed = false;
    const query = searchQuery.trim();

    if (!query) {
      setSearchResults({ query: '', items: [] });
      setSearchLoading(false);
      setSearchError(null);
      return () => {
        disposed = true;
      };
    }

    setSearchLoading(true);
    const timer = window.setTimeout(() => {
      api
        .searchPrompts<PromptSearchResults>({
          query,
          includeLowInfo,
        })
        .then((nextResults) => {
          if (!disposed) {
            setSearchResults(nextResults);
            setSearchError(null);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setSearchError(String(reason));
          }
        })
        .finally(() => {
          if (!disposed) {
            setSearchLoading(false);
          }
        });
    }, 250);

    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [includeLowInfo, searchQuery]);

  return (
    <section className="search-panel" aria-label="prompt 搜索">
      <div className="section-heading">
        <h3>搜索</h3>
        <span>{searchLoading ? '搜索中' : `${searchResults.items.length} 条`}</span>
      </div>
      <div className="search-body">
        <div className="search-row">
          <input
            aria-label="搜索会话标题、prompt 和当前草稿"
            onChange={(event) => setSearchQuery(event.currentTarget.value)}
            placeholder="搜索会话标题、首条 prompt、已发送 prompt、当前草稿"
            type="search"
            value={searchQuery}
          />
          <label className="check-control">
            <input
              checked={hideLowInfo}
              onChange={(event) => onHideLowInfoChange(event.currentTarget.checked)}
              type="checkbox"
            />
            隐藏低信息
          </label>
        </div>
        {searchError ? <p className="error-banner">搜索失败：{searchError}</p> : null}
        <SearchResultsList items={searchResults.items} onSelect={onSelect} />
      </div>
    </section>
  );
}

function SearchResultsList({
  items,
  onSelect,
}: {
  items: PromptSearchResultItem[];
  onSelect: (item: PromptSearchResultItem) => void;
}) {
  if (!items.length) {
    return (
      <div className="history-empty">
        <p>暂无搜索结果</p>
      </div>
    );
  }

  return (
    <div className="search-results" aria-label="搜索结果列表">
      {items.map((item, index) => (
        <button
          className={item.isLowInfo ? 'search-result low-info' : 'search-result'}
          key={`${item.provider}:${item.sessionId}:${item.matchKind}:${index}`}
          onClick={() => onSelect(item)}
          type="button"
        >
          <span>
            <strong>{item.title}</strong>
            <small>
              {item.matchLabel} · {item.providerLabel} · {item.shortSessionId} ·{' '}
              {item.projectName}
            </small>
          </span>
          <em>{item.snippet}</em>
          <small>{formatDateTime(item.sentAt ?? item.updatedAt)}</small>
        </button>
      ))}
    </div>
  );
}

function formatDateTime(value: string | null) {
  if (!value) {
    return '暂无';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
