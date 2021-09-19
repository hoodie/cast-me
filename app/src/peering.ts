import { sendAsCandidate, sendAsOffer, sendAsAnswer, candidates, offers, answers } from './network'

const defaults = {
    polite: false,
};

export interface PeerInterface {
    pc: RTCPeerConnection;
    negotiate(): void;
}

export function initP2P({ polite } = defaults): PeerInterface {
    const pc = new RTCPeerConnection();
    const negotiate = async () => {
        const offer = await pc.createOffer();
        if (pc.signalingState !== 'stable') { return; }
        await pc.setLocalDescription(offer);
        sendAsOffer(pc.localDescription);

    };

    pc.onicecandidate = ({ candidate }) => {
        console.debug({ candidate });
        sendAsCandidate(candidate);
    };
    candidates.subscribe((async candidate => await pc.addIceCandidate(candidate)));

    pc.onnegotiationneeded = async () => {
        console.debug('negotiation needed')
        negotiate();
    };

    offers.subscribe(async offer => {
        console.debug('offer', offer);
        if (pc.signalingState !== 'stable') {
            console.debug('offer but unstable ', { polite })
            if (!polite) { return; }
            await Promise.all([
                pc.setLocalDescription({ type: 'rollback' }),
                pc.setRemoteDescription(offer),
            ]);
        } else {
            console.debug('offer in stable', { polite })
            await pc.setRemoteDescription(offer);
        }
        await pc.setLocalDescription(await pc.createAnswer());
        sendAsAnswer(pc.localDescription);
    });

    answers.subscribe(async answer => {
        console.debug('answer', answer);
        await pc.setRemoteDescription(answer)
    });

    return { pc, negotiate };
}
