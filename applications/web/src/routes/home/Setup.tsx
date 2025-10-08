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
    CircularProgress,
    Grid2 as Grid,
    Table,
    TableBody,
    TableContainer,
    TableHead,
    TableRow,
    TextField
} from "@mui/material";
import {useEffect, useState} from "react";
import useSettings from "../../store/settings";
import SecondaryHeading from "../../components/SecondaryHeading";
import {DataTableCell, StyledPaper} from "../../components/StyledComponents";
import NewIssuerDialog from "./NewIssuerDialog";
import useTariProvider from "../../store/provider";
import {NewIssuerParams} from "../../types";
import {Link, useNavigate} from "react-router-dom";
import useIssuers from "../../store/issuers";
import {StableCoinIssuer} from "../../store/stableCoinIssuer";
import useActiveAccount from "../../store/account";
import IconButton from "@mui/material/IconButton";
import {RefreshOutlined} from "@mui/icons-material";
import {AccountDetails} from "../../components/AccountDetails";
import {CborValue, getCborValueByPath, Substate, TariProvider, TariSigner} from "@tari-project/tarijs-all";
import TariWallet from "../../wallet";
import {ComponentHeader, decodeOotleAddress, substateIdToString, VaultId} from "@tari-project/typescript-bindings";

function SetTemplateForm() {
    const {settings, setTemplate} = useSettings();
    const [currentSettings, setCurrentSettings] = useState(settings);

    return (
        <>

            <form
                onSubmit={(evt) => {
                    evt.preventDefault();
                    if (currentSettings.template) {
                        setTemplate(currentSettings.template);
                    }
                }}
            >
                <Grid size={12}>
                    <p>1. Set the template ID of the issuer template on the current network</p>
                    <TextField
                        name="template ID"
                        placeholder="Template ID"
                        fullWidth
                        onChange={(evt) =>
                            setCurrentSettings({
                                ...currentSettings,
                                template: evt.target.value,
                            })
                        }
                        value={currentSettings.template || ""}
                    />
                </Grid>
                <Grid size={12}>
                    <Button type="submit" disabled={settings.template === currentSettings.template}>
                        Set Template
                    </Button>
                </Grid>

            </form>
        </>
    );
}

function IssuerComponents() {
    const {settings} = useSettings();
    const {provider} = useTariProvider();
    const {getIssuers, addIssuer, setIssuers} = useIssuers();

    const [dialogOpen, setDialogOpen] = useState(false);
    const navigate = useNavigate();
    const [isBusy, setIsBusy] = useState(false);
    const [error, setError] = useState<Error | null>(null);
    const [issuerComponents, setIssuerComponents] = useState<object[] | null>(null);
    const {account} = useActiveAccount();

    if (!account) {
        navigate("/");
        return <></>;
    }

    const decodedAccountAddress = decodeOotleAddress(account!.wallet_address);


    if (provider === null) {
        useEffect(() => {
            navigate("/");
        }, []);
        return <></>;
    }

    useEffect(() => {
        setIsBusy(true);
        provider
            .listSubstates(settings.template, "Component")
            .then((resp) =>
                Promise.all(resp.substates.map((s) => provider.getSubstate(s.substate_id).then((substate) => ({
                    address: substate.address,
                    value: substate.value.Component,
                })))))
            .then((substates) => Promise.all(substates.map((s) => convertToIssuer(provider, s))))
            .then((issuers) => {
                setIssuers(decodedAccountAddress.accountPublicKey, issuers)
            })
            .catch((e: Error) => setError(e))
            .finally(() => setIsBusy(false));
    }, []);

    useEffect(() => {
        if (!isBusy && settings.template && !error) {
            setIssuerComponents(
                (getIssuers(decodedAccountAddress.accountPublicKey) || []).map((issuer) => ({
                    substate_id: issuer.id,
                    version: issuer.version,
                })));
        }
    }, [isBusy, error]);

    function handleOnCreate(data: NewIssuerParams) {
        setIsBusy(true);
        provider!
            .getPublicKey("elgamal_encryption_view_key", 0)
            .then((viewKey) => provider!.createNewIssuer(settings.template!, {...data, viewKey}))
            .then(async (result) => {
                if (result.rejected.isSome()) {
                    throw new Error(`Transaction rejected: ${JSON.stringify(result.rejected.unwrap())}`);
                }
                if (result.onlyFeeAccepted.isSome()) {
                    let [_diff, reason] = result.onlyFeeAccepted.unwrap();
                    throw new Error(`Transaction rejected (fees charged): ${JSON.stringify(reason)}`);
                }

                if (result.accept.isNone()) {
                    throw new Error(`Invariant error: result must be accepted if it is not rejected: ${JSON.stringify(result)}`);
                }

                const diff = result.accept.unwrap();
                const up = diff.upSubstates()
                    .find((up) => up.type == "Component" && (up.substate as ComponentHeader).template_address === settings.template)!;

                let issuer = await convertToIssuer(provider!, {
                    address: {substate_id: up.id, version: up.version},
                    value: up.substate
                });
                addIssuer(decodedAccountAddress.accountPublicKey, issuer);
                navigate(`/issuers/${up.id}`);
            })
            .catch((e) => {
                console.log(e);
                setError(e)
            })
            .finally(() => setIsBusy(false));
    }

    return (
        <>
            <NewIssuerDialog
                open={dialogOpen}
                onClose={() => setDialogOpen(false)}
                onCreate={handleOnCreate}
                isBusy={isBusy}
                error={error}
            />
            <Grid size={12}>
                <p>2. Create a new issuer component, or select an existing one</p>
                <Button onClick={() => setDialogOpen(true)}>Create New Issuer</Button>
                <br/>
                {issuerComponents ? <IssuerTable data={issuerComponents}/> : <CircularProgress/>}
            </Grid>
        </>
    );
}

function IssuerRow({data}: { data: any }) {
    return (
        <>
            <TableRow>
                <DataTableCell width={90} sx={{borderBottom: "none", textAlign: "center"}}>
                    <Link
                        to={`issuers/${substateIdToString(data.substate_id)}`}>{substateIdToString(data.substate_id)}</Link>
                </DataTableCell>
                <DataTableCell>{data.version}</DataTableCell>
            </TableRow>
        </>
    );
}

function IssuerTable({data}: { data: object[] }) {
    return (
        <TableContainer>
            <Table>
                <TableHead>
                    <TableRow>
                        <DataTableCell>Component</DataTableCell>
                        <DataTableCell>Substate Version</DataTableCell>
                    </TableRow>
                </TableHead>
                <TableBody>
                    {data.map((item, key) => (
                        <IssuerRow key={key} data={item}/>
                    ))}
                </TableBody>
            </Table>
        </TableContainer>
    );
}

function InitialSetup() {
    const {provider} = useTariProvider();
    const {settings} = useSettings();
    const navigate = useNavigate();
    const {account, setActiveAccount} = useActiveAccount();

    if (provider === null) {
        useEffect(() => {
            navigate("/");
        }, []);
        return <></>;
    }

    const loadAccount = () => {
        provider.getAccount().then(setActiveAccount);
    };

    // useEffect(() => {
    if (!account) {
        loadAccount();
    }
    // }, [account]);

    if (!account) {
        return <CircularProgress/>;
    }

    return (
        <>
            <Grid size={12}>
                <SecondaryHeading>Setup</SecondaryHeading>
            </Grid>
            <Grid size={12}>
                <StyledPaper>
                    <SetTemplateForm/>
                </StyledPaper>
            </Grid>

            {settings.template && (
                <Grid size={12}>
                    <StyledPaper>
                        <IssuerComponents/>
                    </StyledPaper>
                </Grid>
            )}
            <Grid size={12}>
                <StyledPaper>
                    <IconButton onClick={loadAccount}>
                        <RefreshOutlined/>
                    </IconButton>
                    <p>Connected {provider.providerName()} account: </p>
                    <AccountDetails account={account}/>
                </StyledPaper>
            </Grid>
        </>
    );
}


export async function convertToIssuer<T extends TariProvider, S extends TariSigner>(provider: TariWallet<T, S>, issuer: Substate): Promise<StableCoinIssuer> {
    const {value: component, address} = issuer;
    const structMap = component.body.state as CborValue;
    const vaultId = getCborValueByPath(structMap, "$.token_vault") as string;
    const adminAuthResource = getCborValueByPath(structMap, "$.admin_auth_manager");
    const userAuthResource = getCborValueByPath(structMap, "$.user_auth_manager");
    const {value: vault} = await provider!.getSubstate(vaultId);
    console.log({vaultId, vault});
    if (!vault || !("Vault" in vault)) {
        throw new Error(`${vaultId} is not a vault`);
    }
    const container = vault.Vault.resource_container;
    if (!("Stealth" in container)) {
        throw new Error("Vault is not stealth");
    }

    const wrappedTokenComponent = getCborValueByPath(structMap, "$.wrapped_token") as { vault: VaultId };
    const wrappedVault = wrappedTokenComponent ? await provider!.getSubstate(wrappedTokenComponent.vault) : null;
    const wrappedContainer = (wrappedVault?.value as any)?.Vault.resource_container.Fungible;

    const wrappedToken = wrappedContainer && {
        resource: wrappedContainer.address,
        balance: wrappedContainer.amount,
        ...wrappedTokenComponent,
    };

    return {
        id: address.substate_id,
        version: address.version,
        vault: {
            id: vaultId,
            resourceAddress: container.Stealth.address,
            revealedAmount: container.Stealth.revealed_amount,
        },
        adminAuthResource,
        userAuthResource,
        wrappedToken,
    } as StableCoinIssuer;
}


export default InitialSetup;
