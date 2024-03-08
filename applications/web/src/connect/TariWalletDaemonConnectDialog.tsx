import Box from "@mui/material/Box";
import Dialog from "@mui/material/Dialog";
import Divider from "@mui/material/Divider";
import Stack from "@mui/material/Stack";
import CloseIcon from '@mui/icons-material/Close';
import IconButton from "@mui/material/IconButton";
import Typography from "@mui/material/Typography";
import {QRCodeSVG} from "qrcode.react";
import {useState, useEffect} from "react";
import styles from './TariWalletDaemonConnectDialog.module.css';
import {providers} from "@tariproject/tarijs";
import {Alert, CircularProgress} from "@mui/material";

const {WalletDaemonParameters, WalletDaemonTariProvider, TariPermissions} = providers.walletDaemon;

export interface TariWalletDaemonConnectDialog {
    open: boolean;
    onClose: () => void;
    onConnected: (provider: WalletDaemonTariProvider) => void;
    signalingServerUrl: string;
    permissions: TariPermissions;
    optionalPermissions: TariPermissions;
}

// TODO: hack, onConnection should ideally return the provider once it has connected
const providerHack = {provider: null as any};

export function TariWalletDaemonConnectDialog(props: TariWalletDaemonConnectDialog) {
    const {onClose, open, onConnected} = props;

    const [isCopied, setIsCopied] = useState(false);
    const [fadeClass, setFadeClass] = useState('tariFadeIn');
    const [tokenUrl, setTokenUrl] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<Error | null>(null);

    const onConnection = () => {
        console.log("wallet daemon connected");
        onConnected(providerHack.provider);
        handleClose();
    };

    useEffect(() => {
        if (open) {
            setFadeClass('tariFadeIn');
            const params: WalletDaemonParameters = {
                signalingServerUrl: props.signalingServerUrl,
                permissions: props.permissions,
                optionalPermissions: props.optionalPermissions,
                onConnection,
            };
            setIsLoading(true);
            WalletDaemonTariProvider.build(params)
                .then((provider) => {
                    if (provider.tokenUrl) {
                        setTokenUrl(provider.tokenUrl);
                    }
                    providerHack.provider = provider;
                })
                .catch(setError)
                .finally(() => setIsLoading(false));
            ;
        }
    }, [open]);

    const handleClose = () => {
        onClose();
    };

    const handleCopy = () => {
        navigator.clipboard.writeText(tokenUrl);
        setIsCopied(true);
        setFadeClass('');
        setTimeout(() => {
            setIsCopied(false);
        }, 2000);
    };

    return (
        <Dialog fullWidth={true} onClose={handleClose} open={open}>
            <Box sx={{padding: 4, borderRadius: 4}}>
                <Stack direction="row" justifyContent="space-between" spacing={2}>
                    <Typography style={{fontSize: 24}}>Connect to your Tari Wallet Daemon</Typography>
                    <IconButton aria-label="copy" onClick={handleClose}>
                        <CloseIcon style={{fontSize: 24}}/>
                    </IconButton>
                </Stack>
                <Divider sx={{mt: 3, mb: 1}} variant="middle"/>
                <div className={styles.tariPopupContainer}>
                    <p className={styles.tariText}>
                        Scan the QR code or copy the link below to connect your wallet
                    </p>
                    {isLoading ? <CircularProgress/> : ((!tokenUrl) ?
                        <Alert severity="warning">Unable to contact signalling server</Alert> :
                        <QRCodeSVG value={tokenUrl}/>)}
                    <div className={styles.tariBtnContainer}>
                        <button
                            className={[styles.tariBtn, styles.tariPrimaryBtn].join(' ')}
                            onClick={handleCopy}
                        >
                            {isCopied ? <CheckMark/> : 'Copy Link'}
                        </button>
                        <button
                            className={[styles.tariBtn, styles.tariSecondaryBtn].join(
                                ' '
                            )}
                            onClick={onClose}
                        >
                            Close
                        </button>
                    </div>
                </div>
            </Box>
        </Dialog>
    );
}

const CheckMark = () => {
    return (
        <svg
            className={styles.tariCheckmark}
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 52 52"
        >
            <circle
                className={styles.tariCheckmarkCircle}
                cx="26"
                cy="26"
                r="25"
                fill="none"
            />
            <path
                className={styles.tariCheckmarkCheck}
                fill="none"
                d="M14.1 27.2l7.1 7.2 16.7-16.8"
            />
        </svg>
    );
};
