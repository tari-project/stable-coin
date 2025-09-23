import Box from "@mui/material/Box";
import Dialog from "@mui/material/Dialog";
import Divider from "@mui/material/Divider";
import Stack from "@mui/material/Stack";
import CloseIcon from '@mui/icons-material/Close';
import IconButton from "@mui/material/IconButton";
import Typography from "@mui/material/Typography";
import TariLogo from './content/tari-logo.svg';
import Grid from "@mui/material/Grid";
import Card from "@mui/material/Card";
import CardContent from "@mui/material/CardContent";
import {
    TariPermissions,
    TariProvider,
    TariSigner,
    WalletDaemonFetchParameters,
    WalletDaemonTariProvider,
    WalletDaemonTariSigner,
} from "@tari-project/tarijs-all";
import TariWallet from "../wallet";


// const WALLET_CONNECT_PROJECT_ID = import.meta.env.VITE_WALLET_CONNECT_PROJECT_ID || null;
// const WALLET_DAEMON_ENABLED = import.meta.env.VITE_WALLET_DAEMON_ENABLED || false;

// Minimal permissions for the example site
// But each application will have different permission needs
const walletDaemonPermissions = new TariPermissions();
walletDaemonPermissions
    // Required for createFreeTestCoins
    .addPermission("Admin");
// .addPermission(new TariPermissionKeyList())
// .addPermission(new TariPermissionAccountInfo())
// .addPermission(new TariPermissionTransactionsGet())
// .addPermission(new TariPermissionSubstatesRead())
// .addPermission(new TariPermissionTemplatesRead())
// .addPermission(new TariPermissionTransactionSend());
const walletDaemonOptionalPermissions = new TariPermissions();

export interface WalletSelectionProps {
    open: boolean;
    onConnected: (wallet: TariWallet<TariProvider, TariSigner>) => void;
    onClose: () => void;
}

export function TariWalletSelectionDialog(props: WalletSelectionProps) {
    const {onClose, open, onConnected} = props;

    const handleClose = () => {
        onClose();
    };

    const onWalletDaemonClick = async () => {
        const params: WalletDaemonFetchParameters = {
            permissions: walletDaemonPermissions,
            optionalPermissions: walletDaemonOptionalPermissions,
            serverUrl: "http://127.0.0.1:12019/json_rpc",
        };
        const provider = await WalletDaemonTariProvider.buildFetch(params);
        const signer = await WalletDaemonTariSigner.buildFetchSigner(params);
        onConnected(TariWallet.new(provider, signer));
        handleClose();
    };

    // const onWalletConnectClick = async () => {
    //     const projectId = WALLET_CONNECT_PROJECT_ID;
    //     const walletConnectProvider = new WalletConnectTariProvider(projectId);
    //     handleClose();
    //     await walletConnectProvider.connect();
    //     onConnected(walletConnectProvider);
    // };

    return (
        <Dialog fullWidth={true} onClose={handleClose} open={open}>
            <Box sx={{padding: 4, borderRadius: 4}}>
                <Stack direction="row" justifyContent="space-between" spacing={2}>
                    <Typography style={{fontSize: 24}}>Connect a wallet</Typography>
                    <IconButton aria-label="copy" onClick={handleClose}>
                        <CloseIcon style={{fontSize: 24}}/>
                    </IconButton>
                </Stack>
                <Divider sx={{mt: 3, mb: 3}} variant="middle"/>
                <Grid container spacing={2} justifyContent='center'>
                    {
                        <Grid item xs={4}>
                            <WalletConnectionMethodCard img={TariLogo} text='Tari Wallet Daemon'
                                                        callback={onWalletDaemonClick}></WalletConnectionMethodCard>
                        </Grid>
                    }
                    {/*<Grid item xs={4}>*/}
                    {/*    <WalletConnectionMethodCard*/}
                    {/*        img={WalletConnectLogo}*/}
                    {/*        text="WalletConnect"*/}
                    {/*        callback={onWalletConnectClick}*/}
                    {/*    ></WalletConnectionMethodCard>*/}
                    {/*</Grid>*/}
                </Grid>
            </Box>
        </Dialog>
    );
}

function WalletConnectionMethodCard({img, text, callback}: { img: string, text: string, callback: () => void }) {
    return (
        <Card variant="outlined" elevation={0}
              sx={{mty: 4, padding: 4, borderRadius: 4, width: '175px', height: '175px', cursor: 'pointer'}}>
            <CardContent onClick={async () => {
                await callback()
            }}>
                <Stack direction="column" spacing={2} alignItems='center'>
                    <Box sx={{textAlign: 'center', width: '100%'}}>
                        <img style={{borderRadius: 8, width: '50px', height: '50px'}} src={img}/>
                    </Box>
                    <Typography textAlign='center'>{text}</Typography>
                </Stack>
            </CardContent>
        </Card>
    );
};
