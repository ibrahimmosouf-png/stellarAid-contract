import { create } from 'zustand';

// Explicit type layout for store boundaries
export interface WalletState {
  connectedAddress: string | null;
  walletName: string | null;
  isConnecting: boolean;
  error: string | null;
  connectWallet: (address: string, name: string) => void;
  disconnectWallet: () => void;
  setConnecting: (status: boolean) => void;
  setError: (errorMessage: string | null) => void;
}

// Safely access sessionStorage during hydration checkpoints
const getInitialAddress = (): string | null => {
  if (typeof window !== 'undefined') {
    return sessionStorage.getItem('stellar_connected_address');
  }
  return null;
};

const getInitialWalletName = (): string | null => {
  if (typeof window !== 'undefined') {
    return sessionStorage.getItem('stellar_wallet_name');
  }
  return null;
};

export const useWalletStore = create<WalletState>((set) => ({
  connectedAddress: getInitialAddress(),
  walletName: getInitialWalletName(),
  isConnecting: false,
  error: null,

  connectWallet: (address: string, name: string) => {
    // Task Requirement: Persist connectedAddress in sessionStorage for page refresh resilience
    sessionStorage.setItem('stellar_connected_address', address);
    sessionStorage.setItem('stellar_wallet_name', name);
    
    set({ connectedAddress: address, walletName: name, isConnecting: false, error: null });

    // Task Requirement: Emit wallet:connected events
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('wallet:connected', { detail: { address, name } }));
    }
  },

  disconnectWallet: () => {
    sessionStorage.removeItem('stellar_connected_address');
    sessionStorage.removeItem('stellar_wallet_name');

    set({ connectedAddress: null, walletName: null, isConnecting: false, error: null });

    // Task Requirement: Emit wallet:disconnected events
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('wallet:disconnected'));
    }
  },

  setConnecting: (status: boolean) => set({ isConnecting: status }),
  setError: (errorMessage: string | null) => set({ error: errorMessage, isConnecting: false }),
}));