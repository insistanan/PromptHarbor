export type CustomProviderProtocol =
  | 'openai_chat'
  | 'openai_responses'
  | 'anthropic'
  | 'gemini'
  | 'zhipu_v4';

export type CustomProviderSummary = {
  id: string;
  name: string;
  protocol: CustomProviderProtocol;
  protocolLabel: string;
  baseUrl: string;
  defaultModel: string;
  enabled: boolean;
  supported: boolean;
  secretConfigured: boolean;
};

export type CustomProviderDraft = {
  providerId: string | null;
  name: string;
  protocol: CustomProviderProtocol;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
  enabled: boolean;
  secretConfigured: boolean;
};

export type CustomProviderSaveResult = {
  savedProviderId: string;
  providers: CustomProviderSummary[];
};

export type CustomProviderTestResult = {
  model: string;
  message: string;
  assistantPreview: string;
};

export type PromptOptimizationResult = {
  providerId: string;
  providerName: string;
  model: string;
  optimizedPromptMd: string;
};
