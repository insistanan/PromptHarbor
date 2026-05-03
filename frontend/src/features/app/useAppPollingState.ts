import { useEffect, useMemo, useState } from 'react';
import * as api from '../../api';
import type { AppStatus, SessionList, SessionListItem } from '../../appTypes';
import { findSession } from '../sessions/sessionHelpers';

export function useAppPollingState() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [sessions, setSessions] = useState<SessionList>({
    active: [],
    maybeClosed: [],
    archived: [],
  });
  const [selectedSession, setSelectedSession] = useState<SessionListItem | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;

    const loadStatus = () => {
      api.getAppStatus<AppStatus>()
        .then((nextStatus) => {
          if (!disposed) {
            setStatus(nextStatus);
            setError(null);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    const loadSessions = () => {
      api.listSessions<SessionList>()
        .then((nextSessions) => {
          if (!disposed) {
            setSessions(nextSessions);
            setSelectedSession((current) => {
              if (!current) {
                return nextSessions.active[0] ?? nextSessions.maybeClosed[0] ?? null;
              }

              return findSession(nextSessions, current.provider, current.sessionId);
            });
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    loadStatus();
    loadSessions();
    const timer = window.setInterval(() => {
      loadStatus();
      loadSessions();
    }, 1000);

    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);

  const allSessions = useMemo(
    () => [
      ...sessions.active,
      ...sessions.maybeClosed,
      ...sessions.archived,
    ],
    [sessions],
  );

  return {
    allSessions,
    error,
    selectedSession,
    setError,
    setSelectedSession,
    setSessions,
    setStatus,
    sessions,
    status,
  };
}
