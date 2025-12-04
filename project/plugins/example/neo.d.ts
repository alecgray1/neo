// Neo Plugin Type Definitions

interface ServiceContext {
  state: Record<string, unknown>;
  config: Record<string, unknown>;
}

interface ServiceConfig {
  name: string;
  subscriptions?: string[];
  onStart?: (ctx: ServiceContext) => Promise<void>;
  onStop?: (ctx: ServiceContext) => Promise<void>;
  onEvent?: (ctx: ServiceContext, event: NeoEvent) => Promise<void>;
}

interface NeoEvent {
  type: string;
  source: string;
  data: unknown;
  timestamp: number;
}

interface PinDefinition {
  name: string;
  type: string;
}

interface NodeContext {
  nodeId: string;
  config: Record<string, unknown>;
  inputs: Record<string, unknown>;
  variables: Record<string, unknown>;
  getInput: (name: string) => unknown;
  getConfig: (key: string) => unknown;
  getVariable: (name: string) => unknown;
}

interface NodeConfig {
  name: string;
  category?: string;
  description?: string;
  inputs: PinDefinition[];
  outputs: PinDefinition[];
  pure?: boolean;
  latent?: boolean;
  execute: (ctx: NodeContext) => Promise<Record<string, unknown>>;
}

declare function defineService<T extends ServiceConfig>(config: T): T;
declare function defineNode<T extends NodeConfig>(config: T): T;

declare const Neo: {
  log: {
    error: (msg: string) => void;
    warn: (msg: string) => void;
    info: (msg: string) => void;
    debug: (msg: string) => void;
    trace: (msg: string) => void;
  };
  points: {
    read: (id: string) => Promise<unknown>;
    write: (id: string, value: unknown) => Promise<void>;
  };
  events: {
    emit: (type: string, data: unknown) => void;
  };
  utils: {
    now: () => number;
  };
};
