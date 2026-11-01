// wg.c: the freq probe with an os_workgroup interval (AudioWorkIntervalCreate) around each burst
// usage: wg GAP_MS MODE   MODE 0: none, 1: workgroup interval with a 16 ms deadline
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>
#include <pthread.h>
#include <os/workgroup.h>
#include <mach/mach_time.h>
#include <AudioToolbox/AudioToolbox.h>
#include <mach/mach.h>
#include <mach/thread_policy.h>
static void rt(double ms2abs){thread_time_constraint_policy_data_t p;p.period=(uint32_t)(16*ms2abs);p.computation=(uint32_t)(8*ms2abs);p.constraint=(uint32_t)(16*ms2abs);p.preemptible=1;
 int r=thread_policy_set(pthread_mach_thread_np(pthread_self()),THREAD_TIME_CONSTRAINT_POLICY,(thread_policy_t)&p,THREAD_TIME_CONSTRAINT_POLICY_COUNT); if(r)fprintf(stderr,"rt %d\n",r);}
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC_RAW,&t);return t.tv_sec*1e3+t.tv_nsec*1e-6;}
int main(int argc,char**argv){
  int gap=atoi(argv[1]); int mode=argc>2?atoi(argv[2]):0;
  os_workgroup_interval_t wg=NULL; os_workgroup_join_token_s tok;
  mach_timebase_info_data_t tb; mach_timebase_info(&tb);
  double ms2abs=((double)tb.denom/(double)tb.numer)*1e6;
  if(mode&2) rt(ms2abs);
  if(mode&1){
    wg=AudioWorkIntervalCreate("flashtex-probe", OS_CLOCK_MACH_ABSOLUTE_TIME, NULL);
    if(!wg){fprintf(stderr,"no workgroup\n");return 1;}
    int r=os_workgroup_join(wg,&tok); if(r){fprintf(stderr,"join %d\n",r);return 1;}
  }
  volatile unsigned long sink=0; double all[64]; int na=0;
  for(int r=0;r<40;r++){
    usleep(gap*1000);
    if(mode&1){uint64_t s=mach_absolute_time(); int e=os_workgroup_interval_start(wg,s,s+(uint64_t)(16*ms2abs),NULL); if(e&&r<3)fprintf(stderr,"start %d\n",e);}
    double t0=now();
    unsigned long x=r+1; double marks[4];
    for(int k=0;k<4;k++){ for(long i=0;i<2000000;i++){x=x*6364136223846793005UL+1442695040888963407UL;} marks[k]=now()-t0;}
    if(mode&1) os_workgroup_interval_finish(wg,NULL);
    sink+=x; if(r>=2) all[na++]=marks[3];
    if(r>=2 && getenv("V")) printf("chunks %.2f %.2f %.2f %.2f\n",marks[0],marks[1]-marks[0],marks[2]-marks[1],marks[3]-marks[2]);
  }
  for(int i=0;i<na;i++)for(int j=i+1;j<na;j++)if(all[j]<all[i]){double t=all[i];all[i]=all[j];all[j]=t;}
  printf("gap %d mode %d: 8M-iter p50 %.2f p90 %.2f ms\n",gap,mode,all[na/2],all[na*9/10]);
  return 0;
}
