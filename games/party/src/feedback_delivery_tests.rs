use crate::room::{Phase,Room};
fn finished()->Room{
 let mut room=Room::new("feedback-delivery-unit",16);
 room.connect("alice","Alice","").unwrap();room.connect("bob","Bob","").unwrap();room.start_party("alice").unwrap();
 room.disconnect("alice");room.disconnect("bob");
 for _ in 0..60*400{room.step();if matches!(room.phase,Phase::Results{..}){break;}}
 assert!(matches!(room.phase,Phase::Results{..}));room
}
#[test]
fn preparing_a_rating_or_note_never_claims_storage_and_can_retry_after_failure(){
 let mut room=finished();let seat=room.seat_of("alice");
 let rating=room.prepare_feedback("alice",4,"").unwrap();
 assert_eq!(room.view_for(seat).feedback.unwrap().rated,None);
 assert!(room.prepare_feedback("alice",4,"").is_ok(),"failed persistence remains retryable");
 room.commit_feedback(&rating);assert_eq!(room.view_for(seat).feedback.unwrap().rated,Some(4));
 let note=room.prepare_feedback("alice",0,"  Keep this draft  ").unwrap();
 assert!(note.follow_up);assert_eq!(note.note,"Keep this draft");
 assert!(room.view_for(seat).feedback.is_some(),"the note form remains until storage acknowledges");
 assert!(room.prepare_feedback("alice",0,"Keep this draft").is_ok());
 room.commit_feedback(&note);assert!(room.view_for(seat).feedback.is_none());
 assert!(room.prepare_feedback("alice",0,"duplicate").is_err());
}
#[test]
fn an_old_receipt_cannot_mark_a_different_bout_or_party_as_rated(){
 let mut room=finished();let seat=room.seat_of("alice");let receipt=room.prepare_feedback("alice",5,"").unwrap();
 let mut old=receipt.clone();old.bout.number+=1;room.commit_feedback(&old);assert_eq!(room.view_for(seat).feedback.unwrap().rated,None);
 old=receipt.clone();old.party_id.push_str("-old");room.commit_feedback(&old);assert_eq!(room.view_for(seat).feedback.unwrap().rated,None);
 room.commit_feedback(&receipt);assert_eq!(room.view_for(seat).feedback.unwrap().rated,Some(5));
}
