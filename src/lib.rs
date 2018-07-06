extern crate num;
extern crate num_complex;
extern crate black_scholes;
extern crate rayon;
extern crate fang_oost;
#[macro_use]
#[cfg(test)]
extern crate approx;
#[cfg(test)]
extern crate statrs;

use num_complex::Complex;
use num::traits::{Zero};
use std::f64::consts::PI;
use rayon::prelude::*;




/**For Fang Oost (defined in the paper)*/
fn chi_k(a:f64, b:f64, c:f64, d:f64, u:f64)->f64{
    let iter_s=|x|u*(x-a);
    let exp_d=d.exp();
    let exp_c=c.exp();
    (iter_s(d).cos()*exp_d-iter_s(c).cos()*exp_c+u*iter_s(d).sin()*exp_d-u*iter_s(c).sin()*exp_c)/(1.0+u*u)
}

fn phi_k(a:f64, b:f64, c:f64, d:f64, u:f64, k:usize)->f64{
    let iter_s=|x|u*(x-a);
    if(k==0){d-c} else{(iter_s(d).sin()-iter_s(c).sin())/u}
}

/**This function takes strikes and converts them
into a vector in the x domain.  Intriguinely, I 
don't have to sort the result...*/
fn get_x_from_k(asset:f64, strikes:&Vec<f64>)->Vec<f64>{
    strikes.iter().map(|v|(asset/v).ln()).collect()
}

fn option_price_transform(cf:&Complex<f64>)->Complex<f64>{
    *cf
}


/**
    Fang Oosterlee Approach for an option using Put as the main payoff 
    (better accuracy than a call...use put call parity to get back put).
    Note that Fang Oosterlee's approach works well for a smaller 
    of discrete strike prices such as those in the market.  The 
    constraint is that the smallest and largest values in the x domain
    must be relatively far from the middle values.  This can be 
    "simulated" by adding small and large "K" synthetically.  Due to
    the fact that Fang Oosterlee is able to handle this well, the 
    function takes a vector of strike prices with no requirement that
    the strike prices be equidistant.  All that is required is that
    they are sorted largest to smallest.
    returns in log domain
    http://ta.twi.tudelft.nl/mf/users/oosterle/oosterlee/COS.pdf
    @xValues x values derived from strikes
    @numUSteps number of steps in the complex domain (independent 
    of number of x steps)
    @mOutput a function which determines whether the output is a 
    call or a put.  
    @CF characteristic function of log x around the strike
    @returns vector of prices corresponding with the strikes 
    provided by FangOostCall or FangOostPut
*/

fn fang_oost_generic<T, U, S>(
    num_u:usize, 
    x_values:&Vec<f64>,
    enh_cf:T,
    m_output:U,
    cf:S
)->Vec<f64>
    where T: Fn(&Complex<f64>, &Complex<f64>)->Complex<f64>+std::marker::Sync+std::marker::Send,
    U: Fn(f64, f64, usize)->f64+std::marker::Sync+std::marker::Send,
    S:Fn(&Complex<f64>)->Complex<f64>+std::marker::Sync+std::marker::Send
{
    let x_max=*x_values.last().unwrap();
    let x_min=*x_values.first().unwrap();
    fang_oost::get_expectation_discrete_extended(
        num_u,
        x_values, 
        |u|{
            let cfu=cf(u);
            enh_cf(&cfu, u)
        },
        |u, x, k|phi_k(x_min, x_max, x_min, 0.0, u, k)-chi_k(x_min, x_max, x_min, 0.0, u)
    )
}

pub fn fang_oost_call_price<S>(
    num_u:usize,
    asset:f64,
    strikes:&Vec<f64>,
    rate:f64,
    t_maturity:f64,
    cf:S
)->Vec<f64>
    where S:Fn(&Complex<f64>)->Complex<f64>+std::marker::Sync+std::marker::Send
{
    let discount=(-rate*t_maturity).exp();
    let t_strikes=get_x_from_k(asset, strikes);
    fang_oost_generic(
        num_u, 
        &t_strikes, 
        |cfu, u|option_price_transform(&cfu), 
        |val, x, index|(val-1.0)*discount*strikes[index]+asset,
        cf
    )
}



#[cfg(test)]
mod tests {
    use super::*;
    fn get_fang_oost_k_at_index(
        x_min:f64,
        dk:f64,
        asset:f64,
        index:usize
    )->f64{
        asset*(-x_min+dk*(index as f64)).exp()
    }
   
    fn get_fang_oost_strike(
        x_min:f64,
        x_max:f64,
        asset:f64,
        num_x:usize
    )->Vec<f64>{
        let dx=(x_max-x_min)/(num_x as f64-1.0);
        (0..num_x).map(|index|{
            get_fang_oost_k_at_index(x_min, dx, asset, index)
        }).collect()
    }
    

    #[test]
    fn test_fang_oost_call(){
        let r=0.05;
        let sig=0.3;
        let t=1.0;
        let asset=50.0;
        let bs_cf=|u:&Complex<f64>| ((r-sig*sig*0.5)*t*u+sig*sig*t*u*u*0.5).exp();
        let x_max=5.0;
        let num_x=(2 as usize).pow(10);
        let num_u=64;
        let k_array=get_fang_oost_strike(-x_max, x_max, asset, num_x);
        let my_option_price=fang_oost_call_price(num_u, asset, &k_array, r, t, bs_cf);
        let min_n=num_x/4;
        let max_n=num_x-num_x/4;
        let discount=(-r*t).exp();
        for i in min_n..max_n{
            assert_abs_diff_eq!(
                black_scholes::call(asset, k_array[i], discount, sig),
                my_option_price[i],
                epsilon=0.001
            );
        }
    }

}
