import{randomBytes}from'node:crypto';
import{SyntheticProviderApi}from'./provider-api.mjs';
export function admitSyntheticAccount(value,username){if(!/^rom-probe-[a-f0-9]{24}$/.test(username)||value?.username!==username||!Number.isSafeInteger(value.pk)||value.pk<1||value.pk>999999999||value.is_active!==true||value.type!=='internal'||typeof value.uid!=='string'||!/^[A-Za-z0-9._:-]{1,128}$/.test(value.uid))throw Error('new active synthetic provider account required');return{id:value.pk,username:value.username,uid:value.uid};}
export class ProviderAccountControl{
 #api;
 constructor(environment){this.#api=new SyntheticProviderApi(environment,'account');}
 async create(){const username='rom-probe-'+randomBytes(12).toString('hex'),password=randomBytes(24).toString('base64url'),account=admitSyntheticAccount(await this.#api.request('/api/v3/core/users/','POST',{username,name:'ROM unlinked fixture',is_active:true,type:'internal',path:'rom/identity-fixtures',groups:[],roles:[],attributes:{}}),username);await this.#api.request(`/api/v3/core/users/${account.id}/set_password/`,'POST',{password});if(this.#api.records().at(-1).http_status!==204)throw Error('actual password setup not acknowledged');return{account,credentials:{username,password},controls:this.#api.records()};}
}
