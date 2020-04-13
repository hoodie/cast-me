import { writable } from 'svelte/store'
import { connectReceived, payloadMsg } from './network'

export const oppositePeerId = (() => {
    const { subscribe, update } = writable(false);
    return {
        set: (name) => update(current => {
            console.debug("updating 'connected' to", name);
            return name
        }),
        subscribe
    }
})()

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
