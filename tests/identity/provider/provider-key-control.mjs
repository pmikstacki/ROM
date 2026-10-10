// Synthetic-provider controls. Never print API credentials or private key material.
import{randomBytes}from'node:crypto';
import{SyntheticProviderApi}from'./provider-api.mjs';
const uuid=/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
export function admitSigningKey(value){if(!uuid.test(value?.pk)||value.private_key_available!==true||value.key_type!=='rsa')throw Error('actual synthetic RSA key required');return{id:value.pk,type:value.key_type};}
export function admitJwksKeys(value){if(!Array.isArray(value?.keys)||!value.keys.length||value.keys.length>8)throw Error('bounded original JWKS required');const keys=value.keys.map(key=>key.kid);if(keys.some(key=>typeof key!=='string'||!/^[A-Za-z0-9._:-]{1,256}$/.test(key))||new Set(keys).size!==keys.length)throw Error('bounded original JWKS identifiers required');return keys.sort();}
export class SigningKeyControl{
 #api;
 constructor(environment){this.#api=new SyntheticProviderApi(environment,'key');}
 records(){return this.#api.records();}
 async original(){const value=await this.#api.request('/api/v3/providers/oauth2/1/');if(!uuid.test(value.signing_key))throw Error('original provider signing key absent');return value.signing_key;}
 async generate(){return admitSigningKey(await this.#api.request('/api/v3/crypto/certificatekeypairs/generate/','POST',{common_name:'rom-synthetic-rotation-'+randomBytes(12).toString('hex'),validity_days:1}));}
 async select(id){if(!uuid.test(id))throw Error('closed provider key identifier');const value=await this.#api.request('/api/v3/providers/oauth2/1/','PATCH',{signing_key:id});if(value.signing_key!==id)throw Error('provider signing key patch ineffective');return id;}
}
