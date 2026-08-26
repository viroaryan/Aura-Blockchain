export const RPC_ENDPOINT = process.env.NEXT_PUBLIC_RPC_URL || 'http://127.0.0.1:8545'

export interface NodeInfo {
  chain_id: string
  latest_height: number
  state_root: string
  peer_count: number
  mempool_size: number
  version: string
}

export interface BlockHeader {
  version: number
  chain_id: string
  height: number
  round: number
  prev_hash: string
  merkle_root: string
  state_root: string
  validator_set_hash: string
  timestamp: number
  proposer: string
  signature: string
}

export interface Transaction {
  sender: string
  recipient: string
  amount: number
  fee: number
  nonce: number
  tx_type: string
  payload: number[]
  pubkey: string
  signature: string
}

export interface Block {
  header: BlockHeader
  transactions: Transaction[]
  last_commit_qc?: any
}

export interface Validator {
  address: string
  pubkey: string | null
  staked_amount: number
  is_active: boolean
}

export interface Account {
  balance: number
  nonce: number
  staked_amount: number
  is_validator: boolean
  validator_pubkey: string | null
}

export async function jsonRpcCall<T>(method: string, params: any = {}): Promise<T> {
  try {
    const res = await fetch(RPC_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: Date.now(),
        method,
        params,
      }),
      cache: 'no-store',
    })
    const data = await res.json()
    if (data.error) {
      throw new Error(data.error.message || 'RPC Error')
    }
    return data.result
  } catch (err: any) {
    console.warn(`RPC call to ${method} failed:`, err.message)
    throw err
  }
}

export async function getNodeInfo(): Promise<NodeInfo> {
  return jsonRpcCall<NodeInfo>('getNodeInfo')
}

export async function getBlockHeight(): Promise<number> {
  return jsonRpcCall<number>('getBlockHeight')
}

export async function getBlockByHeight(height: number): Promise<Block> {
  return jsonRpcCall<Block>('getBlockByHeight', { height })
}

export async function getBlockByHash(hash: string): Promise<Block> {
  return jsonRpcCall<Block>('getBlockByHash', { hash })
}

export async function getAccount(address: string): Promise<Account> {
  return jsonRpcCall<Account>('getAccount', { address })
}

export async function getValidators(): Promise<Validator[]> {
  return jsonRpcCall<Validator[]>('getValidators')
}
