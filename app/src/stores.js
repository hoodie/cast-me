import { writable, derived } from 'svelte/store'
import { connectReceived, byeReceived, payloadMsg } from './network'

export const createStore = (name) => {
    const { subscribe, update } = writable(false);
    return {
        set: (value) => update(_current => {
            console.debug(`updating '${name}' to`, value);
            return value 
        }),
        subscribe
    }
};

export const oppositePeerId = createStore('connected');
export const oppositePeerLeftReason = createStore('oppositePeerLeftReason');

export const messageHistory = (() => {
    const { subscribe, update } = writable([]);
    payloadMsg.subscribe(msg => update(history => [...history.slice(-100), msg]))
    return {
        subscribe
    }
})()

connectReceived.subscribe(correspondent => {
    console.debug('connected to', correspondent)
    oppositePeerId.set(correspondent);
});

byeReceived.subscribe(({ reason })=> {
    console.debug('peer left', reason)
    oppositePeerLeftReason.set(reason);
});
