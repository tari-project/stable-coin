//  Copyright 2022. The Tari Project
//
//  Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
//  following conditions are met:
//
//  1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//  disclaimer.
//
//  2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//  following disclaimer in the documentation and/or other materials provided with the distribution.
//
//  3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote
//  products derived from this software without specific prior written permission.
//
//  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
//  INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//  DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//  SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//  SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
//  WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE
//  USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

import "./Home.css";
import Button from "@mui/material/Button";
import {
    Alert,
    Checkbox,
    CircularProgress,
    FormControlLabel,
    FormLabel,
    MenuItem,
    Select,
    TextField
} from "@mui/material";
import {useState} from "react";
import Grid from "@mui/material/Grid";
import useSettings from "../../store/settings.ts";
import {Dialog, DialogTitle} from "@mui/material";
import {Close} from "@mui/icons-material";
import IconButton from "@mui/material/IconButton";
import DialogContent from "@mui/material/DialogContent";
import CloseIcon from "@mui/icons-material/Close";
import Box from "@mui/material/Box";
import {NewIssuerParams} from "../../types.ts";


interface NewIssuerDialogProps {
    open: boolean;
    isBusy: boolean;
    onClose: () => void;
    onCreate: (params: NewIssuerParams) => void;
    error: Error | null
}

function NewIssuerDialog(props: NewIssuerDialogProps) {
    let [state, setState] = useState<NewIssuerParams>({
        initialSupply: "",
        tokenSymbol: "",
        tokenMetadata: {provider_name: ""},
        enableWrappedToken: true,
        viewKey: "",
    });

    return (
        <Dialog open={props.open} onClose={props.onClose} fullWidth={true}>
            <Box sx={{paddingX: 4, borderRadius: 4}}>
                <Box>
                    <DialogTitle sx={{display: 'flex', justifyContent: 'space-between'}}>
                        Create a new issuer
                        <IconButton onClick={props.onClose}><CloseIcon/></IconButton>
                    </DialogTitle>
                </Box>
                <DialogContent>
                    <Grid container spacing={2}>
                        <form onSubmit={(e) => {
                            e.preventDefault();
                            props.onCreate(state)
                        }}>
                            <Grid item xs={12} md={12} lg={12} sx={{paddingBottom: 2}}>
                                <TextField
                                    name="initialSupply"
                                    placeholder="1000000"
                                    disabled={props.isBusy}
                                    label="Initial Supply"
                                    onChange={(e) => {
                                        setState({...state, initialSupply: e.target.value})
                                    }}
                                    value={state.initialSupply || ""}
                                />
                            </Grid>
                            <Grid item xs={12} md={12} lg={12} sx={{paddingBottom: 2}}>
                                <TextField
                                    name="tokenSymbol"
                                    placeholder="xUSD"
                                    disabled={props.isBusy}
                                    label="Token Symbol"
                                    onChange={(e) => {
                                        setState({...state, tokenSymbol: e.target.value})
                                    }}
                                    value={state.tokenSymbol || ""}
                                />
                            </Grid>
                            <Grid item xs={12} md={12} lg={12} sx={{paddingBottom: 2}}>
                                <TextField
                                    name="tokenMetadata"
                                    label="Provider name"
                                    placeholder="Stable4U"
                                    disabled={props.isBusy}
                                    onChange={(e) => {
                                        setState({...state, tokenMetadata: {provider_name: e.target.value}})
                                    }}
                                    value={state.tokenMetadata?.provider_name || ""}
                                />
                            </Grid>
                            <Grid item xs={12} md={12} lg={12} sx={{paddingBottom: 2}}>
                                <FormControlLabel control={
                                    <Checkbox
                                        name="enableWrappedToken"
                                        disabled={props.isBusy}
                                        onChange={(e) => {
                                            setState({...state, enableWrappedToken: e.target.value == 'y'})
                                        }}
                                        defaultChecked
                                    />
                                } label="Enable wrapped token"/>

                            </Grid>
                            <Grid item xs={12} md={12} lg={12}>
                                {props.isBusy ? <CircularProgress/> :
                                    <Button type="submit" disabled={props.isBusy}>
                                        Create
                                    </Button>}
                                {props.error && <Alert severity="error">{props.error.message}</Alert>}
                            </Grid>
                        </form>
                    </Grid>
                </DialogContent>
            </Box>
        </Dialog>
    )
}

export default NewIssuerDialog;
