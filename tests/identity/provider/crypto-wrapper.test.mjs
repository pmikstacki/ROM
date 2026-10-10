import {test} from 'node:test';
import assert from 'node:assert/strict';
import {renderCryptoWrapper,admitLoadedCrypto,requireOwnedBrowserMembership} from './crypto-wrapper.mjs';
test('browser mapping accepts actual direct owned cgroup and its owned launcher child',()=>{
 for(const suffix of ['', '/launcher'])assert.doesNotThrow(()=>requireOwnedBrowserMembership(`0::/rom-identity-${'a'.repeat(32)}${suffix}`));
 for(const raw of ['0::/','0::/other','0::/rom-identity-a','0::/rom-identity-'+ 'a'.repeat(32)+'/unowned'])assert.throws(()=>requireOwnedBrowserMembership(raw));
});
test('private crypto wrapper preserves original assets and exact argument forwarding',()=>{
 const text='#!/bin/sh\nMYDIR="$(dirname $(readlink -f $0))"\nexport WEBKIT_EXEC_PATH="${MYDIR}/bin"\nexport LD_LIBRARY_PATH="${MYDIR}/lib:/nix/store/fixed/lib"\nexec "${MYDIR}/bin/MiniBrowser" "$@"\n';
 const output=renderCryptoWrapper(text,'/var/tmp/runtime/minibrowser-wpe','/var/tmp/crypto/lib:/nix/store/dependency/lib');
 assert.ok(output.includes('MYDIR="/var/tmp/runtime/minibrowser-wpe"'));
 assert.ok(output.includes('LD_LIBRARY_PATH="/var/tmp/crypto/lib:/nix/store/dependency/lib:${MYDIR}/lib:'));
 assert.ok(output.endsWith('exec "${MYDIR}/bin/MiniBrowser" "$@"\n'));
});
test('wrapper fails closed on changed export structure and shell-bearing paths',()=>{
 assert.throws(()=>renderCryptoWrapper('unexpected','/fixed','/fixed/lib'));
 assert.throws(()=>renderCryptoWrapper('','/fixed;evil','/fixed/lib'));
});
test('actual loaded crypto admission needs both selected libraries and rejects old copies',()=>{
 const paths=['/private/lib/libgnutls.so.30','/private/lib/libtasn1.so.6'];
 assert.doesNotThrow(()=>admitLoadedCrypto(paths,paths));
 assert.throws(()=>admitLoadedCrypto(paths.slice(0,1),paths));
 assert.throws(()=>admitLoadedCrypto([...paths,'/old/libgnutls.so.30.0.0'],paths));
});
