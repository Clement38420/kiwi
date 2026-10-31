import type { RpcRequest } from '@/bindings/RpcRequest.ts'
import type { GcsToRpi } from '@/bindings/GcsToRpi.ts'
import type { RpiToGcs } from '@/bindings/RpiToGcs.ts'
import type { RpcResponse } from '@/bindings/RpcResponse.ts'
import type { RpcErrorKind } from '@/bindings/RpcErrorKind.ts'

// Extracts message names from a union of messages `U`
type MessagesOf<U> = U extends unknown ? keyof U : never
// Gets the type of message `K` that comes from a union of messages `U`
type MessageTypeOf<U, K extends PropertyKey> = Extract<U, Record<K, unknown>>[K]
// Remove the trailing `S` from a message name `K` if it ends with `S`, otherwise returns never
type Strip<K, S extends string> = K extends `${infer N}${S}` ? N : never
// Extracts messages from a union of messages `U` that end with `S`
type MessageEndingWith<U, S extends string> = U extends unknown
  ? keyof U extends `${string}${S}`
    ? U
    : never
  : never

export type RpcRequestName = Strip<MessagesOf<GcsToRpi>, 'Req'>
export type RpcResponseMessage = MessageEndingWith<RpiToGcs, 'Res'>
// Gets the request type of RPC message `N`
type ReqTypeOf<N extends RpcRequestName> =
  MessageTypeOf<GcsToRpi, `${N}Req`> extends RpcRequest<infer M> ? M : never
// Gets the response type of RPC message `N`
type ResTypeOf<N extends RpcRequestName> =
  MessageTypeOf<RpiToGcs, `${N}Res`> extends RpcResponse<infer M> ? M : never
// Args needed to build a `N` request
type ReqArgsOf<N extends RpcRequestName> = ReqTypeOf<N> extends null ? [] : [message: ReqTypeOf<N>]

export class RpcError extends Error {
  readonly request: RpcRequestName
  readonly kind: RpcErrorKind

  constructor(request: RpcRequestName, kind: RpcErrorKind) {
    super(`RPC '${request}' failed: ${kind}`)
    this.name = 'RpcError'
    this.request = request
    this.kind = kind
  }
}

interface Pending {
  name: RpcRequestName
  resolve: (value: ResTypeOf<RpcRequestName>) => void
  reject: (reason: RpcError) => void
  timer: ReturnType<typeof setTimeout>
}

export function createRpcClient(send: (message: GcsToRpi) => void, timeoutMs = 1000) {
  const pendingRequests = new Map<number, Pending>()

  function request<N extends RpcRequestName>(
    name: N,
    ...args: ReqArgsOf<N>
  ): Promise<ResTypeOf<N>> {
    const id = crypto.getRandomValues(new Uint32Array(1))[0] ?? 0 // Should never be null
    const payload = args[0] ?? null

    return new Promise<ResTypeOf<N>>((resolve, reject) => {
      const timer = setTimeout(() => {
        pendingRequests.delete(id)
        reject(new RpcError(name, 'TimedOut'))
      }, timeoutMs)
      pendingRequests.set(id, { name, resolve: resolve as (v: unknown) => void, reject, timer })
      send({ [`${name}Req`]: { id, payload } } as unknown as GcsToRpi)
    })
  }

  function handleMessage(message: RpiToGcs): message is RpcResponseMessage {
    const [name, value] = Object.entries(message)[0] ?? []
    if (!name?.endsWith('Res')) return false

    const { id, payload } = value as RpcResponse<ResTypeOf<RpcRequestName>>
    const pending = pendingRequests.get(id)
    if (!pending) return true

    pendingRequests.delete(id)
    clearTimeout(pending.timer)
    pending.resolve(payload)
    return true
  }

  function rejectAll(reason: RpcErrorKind = 'ConnectionClosed') {
    for (const p of pendingRequests.values()) {
      clearTimeout(p.timer)
      p.reject(new RpcError(p.name, reason))
    }
    pendingRequests.clear()
  }

  return {
    request,
    handleMessage,
    rejectAll,
  }
}
