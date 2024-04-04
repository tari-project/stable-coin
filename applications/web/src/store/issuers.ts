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

import { create } from "zustand";
import { StableCoinIssuer } from "./stableCoinIssuer.ts";
import { createJSONStorage, persist } from "zustand/middleware";

export interface Store {
  issuers: {
    [key: string]: StableCoinIssuer[];
  },

  getIssuers(accountPk: string): StableCoinIssuer[] | undefined;

  setIssuers(accountPk: string, issuers: StableCoinIssuer[]): void;

  addIssuer(accountPk: string, issuer: StableCoinIssuer): void;

}

const useIssuers = create<Store>()(persist<Store>((set, get) => ({
  issuers: {},
  getIssuers(accountPk: string) {
    return get().issuers[accountPk];
  },
  setIssuers(accountPk: string, accIssuers: StableCoinIssuer[]) {
    const { issuers } = get();
    issuers[accountPk] = accIssuers;
    set({
      issuers,
    });
  },

  addIssuer(accountPk, issuer) {
    const { issuers } = get();
    if (!issuers[accountPk]) {
      issuers[accountPk] = [];
    }
    issuers[accountPk]!.push(issuer);
    set({ issuers });
  },
}), {
  name: "issuers",
  storage: createJSONStorage(() => window.localStorage),
}));

export default useIssuers;
