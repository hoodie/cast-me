import { writable, Writable } from 'svelte/store'
import { connectReceived, byeReceived, payloadMsg, goFullScreenReceived } from './network'

type CreateWritable<T> = (name: string) => Omit<Writable<T>, 'update'>;

export const createStore: CreateWritable<any> = (name) => {
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

export const iInitiatedTheCall = writable(false);

export const messageHistory = (() => {
    const { subscribe, update } = writable([]);
    payloadMsg.subscribe(msg => {
        console.warn('message', msg);
        update(history => [...history.slice(-100), msg])
    })
    return {
        subscribe
    }
})()

connectReceived.subscribe(correspondent => {
    console.debug('connected to', correspondent)
    oppositePeerId.set(correspondent);
});

byeReceived.subscribe(({ reason }) => {
    console.debug('peer left', reason)
    oppositePeerLeftReason.set(reason);
});

export const goFullScreen = writable(false);
goFullScreenReceived.subscribe((payload) => {
    console.debug('fullscreen', payload)
    goFullScreen.set(payload);
});
