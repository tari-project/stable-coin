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

// import { useEffect, useState } from "react";
import Button from "@mui/material/Button";
import { FormLabel, TextField, Select, MenuItem } from "@mui/material";
import { useState } from "react";
import Grid from "@mui/material/Grid";
import SecondaryHeading from "./SecondaryHeading.tsx";
import { FunctionDef } from "@tariproject/wallet_jrpc_client";

interface Props {
  func: FunctionDef;
  onCall: (data: object) => void;

  badges: string[];
  selectedBadge: string | null;
  onBadgeChange: (badge: string | null) => void;
  components: string[];
  selectedComponent: string | null;
  onComponentChange: (component: string | null) => void;
}

function CallTemplateForm(props: Props) {
  const {
    func,
    onCall,
    badges,
    selectedBadge,
    onBadgeChange,
    components,
    selectedComponent,
    onComponentChange
  } = props;
  const [data, setData] = useState({});

  const isMethod = func.arguments[0]?.name == "self";
  const args = func.arguments.slice(isMethod ? 1 : 0);

  const hasComponents = components.length > 0;

  return (
    <>
      <SecondaryHeading>
        <pre>{func.name} {isMethod ? "method" : "function"}</pre>
      </SecondaryHeading>
      <form
        onSubmit={evt => {
          evt.preventDefault();
          onCall(data);
        }}
      >
        <Grid item xs={12} md={12} lg={12}>
          {isMethod ? (
            <>
              <SelectField
                name="selectedBadge"
                enableSelectNone={true}
                value={selectedBadge}
                items={badges}
                onChange={onBadgeChange}
              />

              <SelectField
                name="selectedComponent"
                value={selectedComponent}
                items={components}
                onChange={onComponentChange}
              />
            </>
          ) : (
            <span />
          )}

          {args.map((arg, i) => (
            <TemplateTextField
              key={i}
              disabled={isMethod && !hasComponents}
              argName={arg.name}
              onChange={value => setData({ ...data, [arg.name]: value })}
            />
          ))}
        </Grid>
        <Grid item xs={12} md={12} lg={12}>
          <Button type="submit">Submit</Button>
        </Grid>
      </form>
    </>
  );
}

const TemplateTextField = ({
  argName,
  onChange,
  disabled
}: {
  argName: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}) => {
  return (
    <>
      <Grid item xs={12} md={12} lg={12}>
        <FormLabel htmlFor={argName}>{argName}</FormLabel>
      </Grid>
      <Grid item xs={12} md={12} lg={12}>
        <TextField
          id={argName}
          name={argName}
          disabled={disabled}
          placeholder={argName}
          onChange={evt => onChange(evt.target.value)}
        />
      </Grid>
    </>
  );
};

const SelectField = ({
  name,
  enableSelectNone,
  value,
  items,
  onChange
}: {
  name: string;
  enableSelectNone?: boolean;
  value: string | null;
  items: string[];
  onChange: (value: string | null) => void;
}) => {
  return (
    <>
      <Grid item xs={12} md={12} lg={12}>
        <FormLabel htmlFor={name}>{name}</FormLabel>
      </Grid>
      <Grid item xs={12} md={12} lg={12}>
        <Select
          name={name}
          value={value || ""}
          disabled={items.length === 0}
          onChange={evt => onChange(evt.target.value)}
        >
          {enableSelectNone && <MenuItem key={0} value = "">None</MenuItem>}
          {items.map((item, i) => (
            <MenuItem key={i+1} value={item}>
              {item}
            </MenuItem>
          ))}
        </Select>
      </Grid>
    </>
  );
};

export default CallTemplateForm;
