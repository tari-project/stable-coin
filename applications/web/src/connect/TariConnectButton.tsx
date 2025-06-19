import * as React from 'react';

import Button from "@mui/material/Button";
import TariLogoWhite from './content/tari-logo-white.svg';
import {TariWalletSelectionDialog} from './TariWalletSelectionDialog';
import useTariProvider from "../store/provider.js";
import {TariProvider, TariSigner} from '@tari-project/tarijs-all';
import TariWallet from "../wallet.ts";

interface Props {
    onConnected?: (provider: TariProvider) => void;
}

export function TariConnectButton(props: Props) {
    const {provider, setProvider} = useTariProvider();
    const {onConnected} = props;
    const [walletSelectionOpen, setWalletSelectionOpen] = React.useState(false);

    const handleConnectClick = () => {
        setWalletSelectionOpen(true);
    };

    const onWalletSelectionClose = () => {
        setWalletSelectionOpen(false);
    };

    const handleOnConnected = (wallet: TariWallet<TariProvider, TariSigner>) => {
        setProvider(wallet);
        onConnected?.(wallet);
    };

    return (
        <>
            <Button variant='contained' onClick={handleConnectClick}>
                <img width="30px" height="30px" src={TariLogoWhite}/>
                <div style={{paddingLeft: '10px'}}>{provider?.isConnected() ? "Connected" : "Connect"}</div>
            </Button>
            <TariWalletSelectionDialog
                open={walletSelectionOpen}
                onClose={onWalletSelectionClose}
                onConnected={handleOnConnected}
            />
        </>
    );
}
