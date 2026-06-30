import { useWalletStore, WalletState } from '../sdk/state/walletStore';

export interface UseWalletReturn extends WalletState {
  isConnected: boolean;
}

/**
 * Custom hook providing a clean interaction boundary for React components 
 * consuming the centralized wallet connection context.
 */
export const useWallet = (): UseWalletReturn => {
  const store = useWalletStore();

  return {
    ...store,
    isConnected: !!store.connectedAddress,
  };
};