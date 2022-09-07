use heed::types::ByteSlice;
use heed::{BytesDecode, RoTxn};

pub use self::facet_distribution::{FacetDistribution, DEFAULT_VALUES_PER_FACET};
pub use self::filter::Filter;
use crate::heed_codec::facet::{ByteSliceRef, FacetGroupKeyCodec, FacetGroupValueCodec};

mod facet_distribution;
mod facet_distribution_iter;
mod facet_range_search;
pub mod facet_sort_ascending;
pub mod facet_sort_descending;
mod filter;

pub(crate) fn get_first_facet_value<'t, BoundCodec>(
    txn: &'t RoTxn,
    db: heed::Database<FacetGroupKeyCodec<ByteSliceRef>, FacetGroupValueCodec>,
    field_id: u16,
) -> heed::Result<Option<BoundCodec::DItem>>
where
    BoundCodec: BytesDecode<'t>,
{
    let mut level0prefix = vec![];
    level0prefix.extend_from_slice(&field_id.to_be_bytes());
    level0prefix.push(0);
    let mut level0_iter_forward =
        db.as_polymorph().prefix_iter::<_, ByteSlice, ByteSlice>(txn, level0prefix.as_slice())?;
    if let Some(first) = level0_iter_forward.next() {
        let (first_key, _) = first?;
        let first_key = FacetGroupKeyCodec::<BoundCodec>::bytes_decode(first_key)
            .ok_or(heed::Error::Encoding)?;
        Ok(Some(first_key.left_bound))
    } else {
        Ok(None)
    }
}
pub(crate) fn get_last_facet_value<'t, BoundCodec>(
    txn: &'t RoTxn,
    db: heed::Database<FacetGroupKeyCodec<ByteSliceRef>, FacetGroupValueCodec>,
    field_id: u16,
) -> heed::Result<Option<BoundCodec::DItem>>
where
    BoundCodec: BytesDecode<'t>,
{
    let mut level0prefix = vec![];
    level0prefix.extend_from_slice(&field_id.to_be_bytes());
    level0prefix.push(0);
    let mut level0_iter_backward = db
        .as_polymorph()
        .rev_prefix_iter::<_, ByteSlice, ByteSlice>(txn, level0prefix.as_slice())?;
    if let Some(last) = level0_iter_backward.next() {
        let (last_key, _) = last?;
        let last_key = FacetGroupKeyCodec::<BoundCodec>::bytes_decode(last_key)
            .ok_or(heed::Error::Encoding)?;
        Ok(Some(last_key.left_bound))
    } else {
        Ok(None)
    }
}
pub(crate) fn get_highest_level<'t>(
    txn: &'t RoTxn<'t>,
    db: heed::Database<FacetGroupKeyCodec<ByteSliceRef>, FacetGroupValueCodec>,
    field_id: u16,
) -> heed::Result<u8> {
    let field_id_prefix = &field_id.to_be_bytes();
    Ok(db
        .as_polymorph()
        .rev_prefix_iter::<_, ByteSlice, ByteSlice>(&txn, field_id_prefix)?
        .next()
        .map(|el| {
            let (key, _) = el.unwrap();
            let key = FacetGroupKeyCodec::<ByteSliceRef>::bytes_decode(key).unwrap();
            key.level
        })
        .unwrap_or(0))
}

#[cfg(test)]
pub(crate) mod tests {
    use rand::{Rng, SeedableRng};
    use roaring::RoaringBitmap;

    use crate::{heed_codec::facet::OrderedF64Codec, update::facet::tests::FacetIndex};

    pub fn get_simple_index() -> FacetIndex<OrderedF64Codec> {
        let index = FacetIndex::<OrderedF64Codec>::new(4, 8, 5);
        let mut txn = index.env.write_txn().unwrap();
        for i in 0..256u16 {
            let mut bitmap = RoaringBitmap::new();
            bitmap.insert(i as u32);
            index.insert(&mut txn, 0, &(i as f64), &bitmap);
        }
        txn.commit().unwrap();
        index
    }
    pub fn get_random_looking_index() -> FacetIndex<OrderedF64Codec> {
        let index = FacetIndex::<OrderedF64Codec>::new(4, 8, 5);
        let mut txn = index.env.write_txn().unwrap();

        let mut rng = rand::rngs::SmallRng::from_seed([0; 32]);
        let keys =
            std::iter::from_fn(|| Some(rng.gen_range(0..256))).take(128).collect::<Vec<u32>>();

        for (_i, key) in keys.into_iter().enumerate() {
            let mut bitmap = RoaringBitmap::new();
            bitmap.insert(key);
            bitmap.insert(key + 100);
            index.insert(&mut txn, 0, &(key as f64), &bitmap);
        }
        txn.commit().unwrap();
        index
    }
}
